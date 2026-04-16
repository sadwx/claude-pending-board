use claude_pending_board_adapters::AdapterRegistry;
use claude_pending_board_core::board::store::StateStore;
use claude_pending_board_core::config::Config;
use claude_pending_board_core::types::Entry;
use claude_pending_board_core::visibility::{VisibilityController, WallClock};
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub store: StateStore,
    pub visibility: VisibilityController,
    pub config: Config,
    pub adapter_registry: AdapterRegistry,
}

pub type SharedState = Arc<Mutex<AppState>>;

impl AppState {
    pub fn new() -> Self {
        let config = Config::load(&Config::default_path());
        let clock = Arc::new(WallClock);
        let visibility = VisibilityController::new(clock, config.clone());
        let adapter_registry = AdapterRegistry::new();

        Self {
            store: StateStore::new(),
            visibility,
            config,
            adapter_registry,
        }
    }

    pub fn entries(&self) -> Vec<Entry> {
        self.store.snapshot()
    }

    pub fn entry_count(&self) -> usize {
        self.store.len()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
