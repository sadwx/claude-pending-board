use crate::config::Config;
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;

/// Abstraction over time so tests can use fake clocks.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// Real wall clock.
pub struct WallClock;

impl Clock for WallClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// The three visibility states of the HUD.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisibilityState {
    Hidden,
    Shown {
        grace_deadline: Option<DateTime<Utc>>,
    },
    CooldownHidden {
        until: DateTime<Utc>,
        seen_add: bool,
        reminding_override: Option<bool>,
    },
}

/// Events that drive the FSM.
#[derive(Debug, Clone)]
pub enum VisibilityEvent {
    EntryAdded {
        board_count: usize,
    },
    EntryRemoved {
        board_count: usize,
    },
    ManualDismiss {
        reminding_override: Option<bool>,
    },
    ManualOpen,
    Tick,
    /// The dismiss confirmation panel just opened in the HUD. The user is
    /// actively interacting — cancel any pending auto-hide grace deadline so
    /// the HUD doesn't vanish mid-countdown. The eventual `ManualDismiss`
    /// (from a button click, Esc, or the countdown firing) is what hides
    /// the HUD, and it does so via the cooldown path the user actually
    /// intended.
    DismissPanelOpened,
}

/// Actions the UI layer should take in response to a state transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisibilityAction {
    ShowHud,
    HideHud,
    UpdateBadge { count: usize },
    None,
}

pub struct VisibilityController {
    state: VisibilityState,
    clock: Arc<dyn Clock>,
    config: Config,
}

impl VisibilityController {
    pub fn new(clock: Arc<dyn Clock>, config: Config) -> Self {
        Self {
            state: VisibilityState::Hidden,
            clock,
            config,
        }
    }

    pub fn state(&self) -> &VisibilityState {
        &self.state
    }

    pub fn update_config(&mut self, config: Config) {
        self.config = config;
    }

    /// Process an event and return the action the UI should take.
    pub fn handle(&mut self, event: VisibilityEvent) -> VisibilityAction {
        let now = self.clock.now();

        match (&mut self.state, event) {
            // --- Hidden state ---
            (VisibilityState::Hidden, VisibilityEvent::EntryAdded { board_count }) => {
                if board_count > 0 {
                    self.state = VisibilityState::Shown {
                        grace_deadline: None,
                    };
                    VisibilityAction::ShowHud
                } else {
                    VisibilityAction::None
                }
            }
            (VisibilityState::Hidden, VisibilityEvent::ManualOpen) => {
                self.state = VisibilityState::Shown {
                    grace_deadline: None,
                };
                VisibilityAction::ShowHud
            }
            (VisibilityState::Hidden, _) => VisibilityAction::None,

            // --- Shown state ---
            (
                VisibilityState::Shown { grace_deadline },
                VisibilityEvent::EntryAdded { board_count },
            ) => {
                *grace_deadline = None;
                VisibilityAction::UpdateBadge { count: board_count }
            }
            (
                VisibilityState::Shown { grace_deadline },
                VisibilityEvent::EntryRemoved { board_count },
            ) => {
                if board_count == 0 {
                    let deadline = now + Duration::seconds(self.config.auto_hide_grace_secs as i64);
                    *grace_deadline = Some(deadline);
                }
                VisibilityAction::UpdateBadge { count: board_count }
            }
            (
                VisibilityState::Shown { .. },
                VisibilityEvent::ManualDismiss { reminding_override },
            ) => {
                let until = now + Duration::minutes(self.config.cooldown_minutes as i64);
                self.state = VisibilityState::CooldownHidden {
                    until,
                    seen_add: false,
                    reminding_override,
                };
                VisibilityAction::HideHud
            }
            (VisibilityState::Shown { grace_deadline, .. }, VisibilityEvent::Tick) => {
                if let Some(deadline) = grace_deadline {
                    if now >= *deadline {
                        self.state = VisibilityState::Hidden;
                        return VisibilityAction::HideHud;
                    }
                }
                VisibilityAction::None
            }
            (VisibilityState::Shown { .. }, VisibilityEvent::ManualOpen) => VisibilityAction::None,
            (VisibilityState::Shown { grace_deadline }, VisibilityEvent::DismissPanelOpened) => {
                // User is mid-interaction. Cancel any pending grace timer
                // so the HUD doesn't vanish before they commit.
                *grace_deadline = None;
                VisibilityAction::None
            }

            // --- CooldownHidden state ---
            (
                VisibilityState::CooldownHidden { seen_add, .. },
                VisibilityEvent::EntryAdded { board_count },
            ) => {
                *seen_add = true;
                VisibilityAction::UpdateBadge { count: board_count }
            }
            (
                VisibilityState::CooldownHidden { .. },
                VisibilityEvent::EntryRemoved { board_count },
            ) => VisibilityAction::UpdateBadge { count: board_count },
            (VisibilityState::CooldownHidden { .. }, VisibilityEvent::ManualOpen) => {
                self.state = VisibilityState::Shown {
                    grace_deadline: None,
                };
                VisibilityAction::ShowHud
            }
            (
                VisibilityState::CooldownHidden {
                    until,
                    seen_add,
                    reminding_override,
                },
                VisibilityEvent::Tick,
            ) => {
                if now >= *until {
                    let should_remind = reminding_override.unwrap_or(self.config.reminding_enabled);
                    if should_remind && *seen_add {
                        self.state = VisibilityState::Shown {
                            grace_deadline: None,
                        };
                        VisibilityAction::ShowHud
                    } else {
                        self.state = VisibilityState::Hidden;
                        VisibilityAction::HideHud
                    }
                } else {
                    VisibilityAction::None
                }
            }
            (VisibilityState::CooldownHidden { .. }, VisibilityEvent::ManualDismiss { .. }) => {
                VisibilityAction::None
            }
            (VisibilityState::CooldownHidden { .. }, VisibilityEvent::DismissPanelOpened) => {
                // Defensive: panel can't be open while HUD is hidden, but
                // accept the event as a no-op rather than panicking.
                VisibilityAction::None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct FakeClock {
        now: Mutex<DateTime<Utc>>,
    }

    impl FakeClock {
        fn new(now: DateTime<Utc>) -> Arc<Self> {
            Arc::new(Self {
                now: Mutex::new(now),
            })
        }

        fn advance(&self, duration: Duration) {
            let mut now = self.now.lock().unwrap();
            *now += duration;
        }
    }

    impl Clock for FakeClock {
        fn now(&self) -> DateTime<Utc> {
            *self.now.lock().unwrap()
        }
    }

    fn default_config() -> Config {
        Config::default()
    }

    fn t0() -> DateTime<Utc> {
        "2026-04-16T10:00:00Z".parse().unwrap()
    }

    #[test]
    fn test_first_entry_shows_hud() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock, default_config());
        let action = ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        assert_eq!(action, VisibilityAction::ShowHud);
        assert!(matches!(ctrl.state(), VisibilityState::Shown { .. }));
    }

    #[test]
    fn test_additional_add_while_shown_does_not_reshow() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock, default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        let action = ctrl.handle(VisibilityEvent::EntryAdded { board_count: 2 });
        assert_eq!(action, VisibilityAction::UpdateBadge { count: 2 });
    }

    #[test]
    fn test_board_empty_starts_grace_timer() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock.clone(), default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        let action = ctrl.handle(VisibilityEvent::EntryRemoved { board_count: 0 });
        assert_eq!(action, VisibilityAction::UpdateBadge { count: 0 });
        assert!(matches!(
            ctrl.state(),
            VisibilityState::Shown {
                grace_deadline: Some(_)
            }
        ));
    }

    #[test]
    fn test_grace_timer_expired_hides_hud() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock.clone(), default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        ctrl.handle(VisibilityEvent::EntryRemoved { board_count: 0 });
        clock.advance(Duration::seconds(3));
        let action = ctrl.handle(VisibilityEvent::Tick);
        assert_eq!(action, VisibilityAction::HideHud);
        assert_eq!(*ctrl.state(), VisibilityState::Hidden);
    }

    #[test]
    fn test_new_add_during_grace_cancels_timer() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock.clone(), default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        ctrl.handle(VisibilityEvent::EntryRemoved { board_count: 0 });
        let action = ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        assert_eq!(action, VisibilityAction::UpdateBadge { count: 1 });
        assert!(matches!(
            ctrl.state(),
            VisibilityState::Shown {
                grace_deadline: None
            }
        ));
    }

    #[test]
    fn test_manual_dismiss_enters_cooldown() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock, default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        let action = ctrl.handle(VisibilityEvent::ManualDismiss {
            reminding_override: None,
        });
        assert_eq!(action, VisibilityAction::HideHud);
        assert!(matches!(
            ctrl.state(),
            VisibilityState::CooldownHidden { .. }
        ));
    }

    #[test]
    fn test_add_during_cooldown_sets_seen_flag_but_no_show() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock, default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        ctrl.handle(VisibilityEvent::ManualDismiss {
            reminding_override: None,
        });
        let action = ctrl.handle(VisibilityEvent::EntryAdded { board_count: 2 });
        assert_eq!(action, VisibilityAction::UpdateBadge { count: 2 });
        match ctrl.state() {
            VisibilityState::CooldownHidden { seen_add, .. } => assert!(*seen_add),
            _ => panic!("expected CooldownHidden"),
        }
    }

    #[test]
    fn test_cooldown_expiry_with_reminding_on_and_seen_add_shows_hud() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock.clone(), default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        ctrl.handle(VisibilityEvent::ManualDismiss {
            reminding_override: None,
        });
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 2 });
        clock.advance(Duration::minutes(16));
        let action = ctrl.handle(VisibilityEvent::Tick);
        assert_eq!(action, VisibilityAction::ShowHud);
    }

    #[test]
    fn test_cooldown_expiry_no_seen_add_stays_hidden() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock.clone(), default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        ctrl.handle(VisibilityEvent::ManualDismiss {
            reminding_override: None,
        });
        clock.advance(Duration::minutes(16));
        let action = ctrl.handle(VisibilityEvent::Tick);
        assert_eq!(action, VisibilityAction::HideHud);
        assert_eq!(*ctrl.state(), VisibilityState::Hidden);
    }

    #[test]
    fn test_cooldown_expiry_reminding_disabled_stays_hidden() {
        let clock = FakeClock::new(t0());
        let mut config = default_config();
        config.reminding_enabled = false;
        let mut ctrl = VisibilityController::new(clock.clone(), config);
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        ctrl.handle(VisibilityEvent::ManualDismiss {
            reminding_override: None,
        });
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 2 });
        clock.advance(Duration::minutes(16));
        let action = ctrl.handle(VisibilityEvent::Tick);
        assert_eq!(action, VisibilityAction::HideHud);
        assert_eq!(*ctrl.state(), VisibilityState::Hidden);
    }

    #[test]
    fn test_override_wake_me_forces_reshow() {
        let clock = FakeClock::new(t0());
        let mut config = default_config();
        config.reminding_enabled = false;
        let mut ctrl = VisibilityController::new(clock.clone(), config);
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        ctrl.handle(VisibilityEvent::ManualDismiss {
            reminding_override: Some(true),
        });
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 2 });
        clock.advance(Duration::minutes(16));
        let action = ctrl.handle(VisibilityEvent::Tick);
        assert_eq!(action, VisibilityAction::ShowHud);
    }

    #[test]
    fn test_override_stay_silent_suppresses_reshow() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock.clone(), default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        ctrl.handle(VisibilityEvent::ManualDismiss {
            reminding_override: Some(false),
        });
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 2 });
        clock.advance(Duration::minutes(16));
        let action = ctrl.handle(VisibilityEvent::Tick);
        assert_eq!(action, VisibilityAction::HideHud);
        assert_eq!(*ctrl.state(), VisibilityState::Hidden);
    }

    #[test]
    fn test_manual_open_during_cooldown_shows_hud() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock, default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        ctrl.handle(VisibilityEvent::ManualDismiss {
            reminding_override: None,
        });
        let action = ctrl.handle(VisibilityEvent::ManualOpen);
        assert_eq!(action, VisibilityAction::ShowHud);
        assert!(matches!(ctrl.state(), VisibilityState::Shown { .. }));
    }

    #[test]
    fn test_dismiss_panel_opened_cancels_grace_deadline() {
        // Repro for the per-entry-X → header-X race: dismissing the last
        // entry sets a grace deadline; opening the dismiss panel must
        // cancel it so a Tick can't auto-hide the HUD mid-countdown.
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock.clone(), default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        ctrl.handle(VisibilityEvent::EntryRemoved { board_count: 0 });
        assert!(matches!(
            ctrl.state(),
            VisibilityState::Shown {
                grace_deadline: Some(_)
            }
        ));

        let action = ctrl.handle(VisibilityEvent::DismissPanelOpened);
        assert_eq!(action, VisibilityAction::None);
        assert!(matches!(
            ctrl.state(),
            VisibilityState::Shown {
                grace_deadline: None
            }
        ));

        // Even after the grace window would have expired, a Tick is now a
        // no-op — the HUD stays Shown so the user can finish interacting
        // with the dismiss panel.
        clock.advance(Duration::seconds(60));
        let action = ctrl.handle(VisibilityEvent::Tick);
        assert_eq!(action, VisibilityAction::None);
        assert!(matches!(ctrl.state(), VisibilityState::Shown { .. }));
    }

    #[test]
    fn test_manual_dismiss_after_panel_opened_still_enters_cooldown() {
        // After cancelling the grace timer via DismissPanelOpened, the
        // eventual ManualDismiss (countdown / button / Esc) should still
        // transition to CooldownHidden — that's what the user actually
        // intended by clicking the header X.
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock, default_config());
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        ctrl.handle(VisibilityEvent::EntryRemoved { board_count: 0 });
        ctrl.handle(VisibilityEvent::DismissPanelOpened);

        let action = ctrl.handle(VisibilityEvent::ManualDismiss {
            reminding_override: None,
        });
        assert_eq!(action, VisibilityAction::HideHud);
        assert!(matches!(
            ctrl.state(),
            VisibilityState::CooldownHidden { .. }
        ));
    }

    #[test]
    fn test_dismiss_panel_opened_in_other_states_is_noop() {
        let clock = FakeClock::new(t0());
        let mut ctrl = VisibilityController::new(clock, default_config());
        // Hidden → no-op (catch-all already handles this; assert it
        // doesn't transition or panic).
        let action = ctrl.handle(VisibilityEvent::DismissPanelOpened);
        assert_eq!(action, VisibilityAction::None);
        assert!(matches!(ctrl.state(), VisibilityState::Hidden));

        // CooldownHidden → no-op (defensive; shouldn't happen in practice).
        ctrl.handle(VisibilityEvent::EntryAdded { board_count: 1 });
        ctrl.handle(VisibilityEvent::ManualDismiss {
            reminding_override: None,
        });
        let action = ctrl.handle(VisibilityEvent::DismissPanelOpened);
        assert_eq!(action, VisibilityAction::None);
        assert!(matches!(
            ctrl.state(),
            VisibilityState::CooldownHidden { .. }
        ));
    }
}
