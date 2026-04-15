# Tasks

## 1. Workspace setup

- [ ] 1.1 Create Cargo workspace at repo root with `crates/core`, `crates/adapters`, `crates/app` members
- [ ] 1.2 Pin Rust edition 2021 and MSRV in `rust-toolchain.toml`
- [ ] 1.3 Add `.gitignore` for Rust (`target/`, `.DS_Store`, IDE files) and Tauri (`src-tauri/target/`)
- [ ] 1.4 Add `rustfmt.toml` and `clippy.toml` with house style (4-space indent, `-D warnings` for clippy)
- [ ] 1.5 Commit empty scaffolding and tag `v0.0.0-scaffold`

## 2. `core` crate foundations

- [ ] 2.1 Define domain types in `core::types`: `Entry`, `EntryState` (`Live | Stale`), `NotificationType`, `SessionId`, `TerminalMatch`, `ConfigSnapshot`
- [ ] 2.2 Implement `core::board::parser` for the three op shapes (`add`, `clear`, `stale`); unit tests for valid, malformed, unknown, mixed, and UTF-8 edge cases
- [ ] 2.3 Implement `core::board::store::StateStore` with `apply`, `snapshot`, and sorting (type priority then ts desc); unit tests covering last-write-wins, unknown-clear no-op, and the full sort rule
- [ ] 2.4 Implement `core::board::watcher::BoardWatcher` using the `notify` crate with 50 ms debounce, cursor-tracked reads, graceful handling of file deletion and re-creation
- [ ] 2.5 Implement `core::board::compaction` for startup and threshold-triggered compaction (5 MB / 10 000 lines); atomic write-to-tmp + rename; unit tests for round-trip
- [ ] 2.6 Define `core::terminal::TerminalAdapter` trait and `TerminalMatch` DTO
- [ ] 2.7 Implement `core::terminal::ancestor_walk` using `sysinfo` with depth cap and cycle detection; unit tests against a mock process tree
- [ ] 2.8 Implement `core::config::Config` load / save / hot reload via `notify`; default values from the design doc; unit tests for invalid TOML fall-back

## 3. Visibility state machine

- [ ] 3.1 Introduce a `core::visibility::Clock` trait so timers can be mocked in tests
- [ ] 3.2 Implement `core::visibility::VisibilityController` with states `Hidden`, `Shown`, `CooldownHidden { until, seen_add, reminding_override }`
- [ ] 3.3 Write exhaustive state-transition unit tests: 0→1 auto-show, last-cleared + grace auto-hide, manual dismiss → cooldown, cooldown + seen_add → shown, cooldown + !seen_add → hidden, cooldown + reminding=off → hidden, manual_open cancels cooldown, new add resets grace timer
- [ ] 3.4 Wire the 2-second grace timer and 15-minute cooldown timer to the `Clock` trait
- [ ] 3.5 Unit test reminder override per-dismiss: click "Wake me" = override=Some(true); click "Stay silent" = override=Some(false); countdown expiry = use global setting

## 4. Reaper

- [ ] 4.1 Implement `core::reaper::Reaper` tick function reading `~/.claude/sessions/*.json` and the OS process list
- [ ] 4.2 Implement the pid-alive + session-file-match dual check and the `stale` promotion write
- [ ] 4.3 Implement 24-hour stale expiry that drops entries during compaction
- [ ] 4.4 Unit tests against a mock process table and mock fs: live, dead, recycled, mismatched, expired-stale
- [ ] 4.5 Wrap the reaper task in a tokio `AbortHandle` supervisor that restarts on panic

## 5. `adapters` crate

- [ ] 5.1 Implement `adapters::wezterm::WezTermAdapter` with `detect`, `focus_pane`, `spawn_resume`
- [ ] 5.2 Implement the process ancestor-to-pane matching logic for WezTerm using `wezterm cli list --format json`
- [ ] 5.3 Implement `adapters::iterm2::ITerm2Adapter` gated by `#[cfg(target_os = "macos")]` with `osascript` driver
- [ ] 5.4 Implement tty-matching logic for iTerm2 session resolution
- [ ] 5.5 Add `#[ignore]` contract tests per adapter that require the real terminal to be running; document the `cargo test -- --ignored` invocation

## 6. Hook scripts

- [ ] 6.1 Write `scripts/pending_hook.ps1` (Windows) handling `Notification`, `UserPromptSubmit`, and `Stop` events
- [ ] 6.2 Implement Windows ancestor walking in PowerShell via `Get-CimInstance Win32_Process`
- [ ] 6.3 Write `scripts/pending_hook.sh` (macOS / Linux) handling the same three events
- [ ] 6.4 Implement POSIX ancestor walking using `ps -o ppid=` (macOS) and `/proc/<pid>/stat` (Linux)
- [ ] 6.5 Wrap both scripts with top-level try/catch that logs to `~/.claude/pending/logs/hook-errors.log` and always exits 0
- [ ] 6.6 Add manual test instructions in `scripts/README.md` (how to pipe a sample JSON payload to each script to verify)

## 7. Tauri app skeleton

- [ ] 7.1 Create `crates/app/src-tauri` with Tauri 2 scaffold, `tauri-plugin-autostart`, `tauri-plugin-single-instance`, `tauri-plugin-positioner`
- [ ] 7.2 Wire tray icon and basic tray context menu (Open, Settings…, Quit)
- [ ] 7.3 Expose Tauri commands: `list_entries`, `focus_entry`, `dismiss_hud`, `open_settings`, `apply_config`, `install_hooks`, `uninstall_hooks`
- [ ] 7.4 Boot `BoardWatcher`, `StateStore`, `VisibilityController`, and `Reaper` as tokio tasks on startup
- [ ] 7.5 Emit Tauri events from backend to frontend on `StateStore` changes and `VisibilityController` transitions

## 8. HUD window

- [ ] 8.1 Create the fixed 380×440 non-resizable, draggable, non-activating HUD window with platform-specific no-activate flags (`WS_EX_NOACTIVATE`, `NSWindow.canBecomeKey = false`, `_NET_WM_STATE_ABOVE`)
- [ ] 8.2 Implement the HUD HTML/CSS in Catppuccin Mocha palette matching the locked pixel spec (44 px header, 52 px entry rows, 3 px accent border per type, section dividers)
- [ ] 8.3 Wire Tauri events to the TS-lite front-end to re-render on state change
- [ ] 8.4 Implement entry click handler → `focus_entry` command → success / inline-error banner
- [ ] 8.5 Implement right-click context menu per entry: open transcript, copy session id, hide entry
- [ ] 8.6 Implement the status-dot heartbeat (unread indicator) driven by scroll position
- [ ] 8.7 Implement drag-to-move and persist window position to `config.toml`
- [ ] 8.8 Validate saved window position against current monitor bounds on restore; fall back to tray-anchor default if off-screen

## 9. Dismiss confirmation panel

- [ ] 9.1 Build the confirmation panel layout (header remains, list area replaced) with the locked v5 design
- [ ] 9.2 Implement the 5-second countdown that ticks inside the default button as `<label> · Ns`
- [ ] 9.3 Implement Esc, click-outside, and countdown-expiry → default-action commit
- [ ] 9.4 Implement click-either-button → cancel countdown, commit chosen action, apply reminding override
- [ ] 9.5 Implement the split-line helper captions (`Choose this to wake me / after N minutes` and `Choose this to stay silent`), reading `N` from live config
- [ ] 9.6 Add a "Skip dismiss confirmation" short-circuit when the corresponding Setting is on

## 10. Settings window

- [ ] 10.1 Create a second, separate Tauri window for Settings (resizable, taskbar-visible)
- [ ] 10.2 Implement the form for all Section 5/7 settings (cooldown, reminding, grace, countdown, skip-confirm, default adapter, HUD position reset)
- [ ] 10.3 Wire form → `apply_config` Tauri command → write `config.toml` → `ConfigWatcher` reloads
- [ ] 10.4 Implement the "Install hooks" button: back up `settings.json`, merge hooks, show diff, commit
- [ ] 10.5 Implement the "Uninstall hooks" button (inverse)
- [ ] 10.6 Surface first-run banner if hooks are not yet installed

## 11. Claude Code plugin

- [ ] 11.1 Create `plugin/` directory with `.claude-plugin/plugin.json` registering the three hooks
- [ ] 11.2 Copy `scripts/pending_hook.ps1` and `pending_hook.sh` into `plugin/hooks/` as part of the build
- [ ] 11.3 Write `plugin/commands/pending-board.md` implementing the `/pending-board` slash command (`status`, `install`, `doctor`, `hooks-uninstall`)
- [ ] 11.4 Write `plugin/README.md` explaining the plugin's relationship to the Tauri app
- [ ] 11.5 Publish plugin to a dedicated GitHub repo suitable for `/plugin marketplace add`

## 12. Documentation

- [ ] 12.1 Write `README.md` at the repo root: what it is, screenshot, quick start, feature list
- [ ] 12.2 Write `INSTALL.md` covering both install paths (plugin + Settings button), per-OS prerequisites, and `/pending-board doctor`
- [ ] 12.3 Write `docs/release-checklist.md` with the manual UX checklist from the design
- [ ] 12.4 Add inline doc comments to the public API of `crates/core`

## 13. CI and release plumbing

- [ ] 13.1 Add GitHub Actions workflow: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`
- [ ] 13.2 Add matrix build job for Windows / macOS / Linux producing Tauri artifacts
- [ ] 13.3 Add adapter contract job that installs WezTerm (Windows / Linux) and runs `cargo test -- --ignored`
- [ ] 13.4 Add a GitHub Release workflow on tag push that uploads signed Tauri binaries
- [ ] 13.5 Draft the v0.1.0 release notes

## 14. Integration and release

- [ ] 14.1 Write `tests/end_to_end.rs` with golden-path, compaction-roundtrip, and real-clock visibility integration tests
- [ ] 14.2 Add a Tauri app smoke test that boots the app and calls `list_entries` via `tauri::test`
- [ ] 14.3 Walk through the manual release checklist on a fresh Windows install and a fresh macOS install
- [ ] 14.4 Fix any issues uncovered by the checklist
- [ ] 14.5 Tag and publish `v0.1.0`
