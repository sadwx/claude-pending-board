# Phase 3: Tauri App & UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Tauri 2 desktop app that boots the core services (BoardWatcher, StateStore, VisibilityController, Reaper), presents the HUD and Settings windows, and wires everything together into a working tray application.

**Architecture:** The `crates/app/` crate becomes a Tauri 2 application. The Rust backend manages shared `AppState` (behind `Arc<Mutex<>>`) containing the `StateStore`, `VisibilityController`, `Config`, and `AdapterRegistry`. A background tokio task receives ops from the `BoardWatcher`, updates the store, runs the visibility FSM, and emits Tauri events to the frontend. The frontend is vanilla HTML/CSS/JS (no framework) split into two windows: the HUD (`ui/hud/`) and Settings (`ui/settings/`). Communication is via `tauri.invoke()` for commands and `tauri.event.listen()` for backend-pushed updates.

**Tech Stack:** Tauri 2, tauri-plugin-single-instance, tauri-plugin-autostart, tauri-plugin-positioner, tokio, serde_json. Frontend: vanilla HTML5, CSS3 (Catppuccin Mocha palette), ES modules with Tauri JS API.

---

## File Structure

```
crates/app/
├── Cargo.toml              # Tauri + workspace deps
├── build.rs                # tauri_build::build()
├── tauri.conf.json         # App config: windows, identifiers, CSP
├── src/
│   ├── main.rs             # Entry point: Builder setup
│   ├── state.rs            # AppState, SharedState type alias
│   ├── commands.rs         # Tauri commands: list_entries, focus_entry, etc.
│   ├── tray.rs             # Tray icon + context menu
│   └── services.rs         # Boot and run background services
├── ui/
│   ├── hud/
│   │   ├── index.html      # HUD window markup
│   │   ├── style.css       # Catppuccin Mocha HUD styles
│   │   └── main.js         # HUD logic: render, click, dismiss
│   └── settings/
│       ├── index.html      # Settings window markup
│       ├── style.css       # Settings styles
│       └── main.js         # Settings logic: form, save
└── icons/
    └── icon.png            # 256x256 tray icon (placeholder)
```

---

## Task 1: Tauri App Scaffold

**Files:**
- Modify: `crates/app/Cargo.toml`
- Create: `crates/app/build.rs`
- Create: `crates/app/tauri.conf.json`
- Create: `crates/app/src/main.rs`
- Create: `crates/app/ui/hud/index.html` (placeholder)
- Create: `crates/app/icons/icon.png` (placeholder)

- [ ] **Step 1: Replace `crates/app/Cargo.toml`**

```toml
[package]
name = "claude-pending-board-app"
edition.workspace = true
version.workspace = true
license.workspace = true

[dependencies]
claude-pending-board-core = { path = "../core" }
claude-pending-board-adapters = { path = "../adapters" }
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-single-instance = "2"
tauri-plugin-autostart = "2"
tauri-plugin-positioner = { version = "2", features = ["tray-icon"] }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

- [ ] **Step 2: Create `crates/app/build.rs`**

```rust
fn main() {
    tauri_build::build();
}
```

- [ ] **Step 3: Create `crates/app/tauri.conf.json`**

```json
{
  "$schema": "https://raw.githubusercontent.com/nickel-org/nickel.rs/master/schema/tauri.schema.json",
  "productName": "Claude Pending Board",
  "version": "0.1.0",
  "identifier": "com.claude-pending-board.app",
  "build": {
    "frontendDist": "../ui/hud"
  },
  "app": {
    "withGlobalTauri": true,
    "windows": [],
    "security": {
      "csp": "default-src 'self'; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline'"
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/icon.png"
    ]
  }
}
```

- [ ] **Step 4: Create placeholder `crates/app/ui/hud/index.html`**

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Claude Pending Board</title>
</head>
<body>
  <h1>Claude Pending Board</h1>
  <p>HUD loading...</p>
</body>
</html>
```

- [ ] **Step 5: Create a placeholder icon**

Create a minimal 32x32 PNG at `crates/app/icons/icon.png`. Use a simple solid color square. The actual icon can be refined later.

Run: `python3 -c "import struct,zlib;d=b'\\x89PNG\\r\\n\\x1a\\n';ihdr=struct.pack('>IIBBBBB',32,32,8,2,0,0,0);ic=b'IHDR'+ihdr;d+=struct.pack('>I',len(ihdr))+ic+struct.pack('>I',zlib.crc32(ic)&0xffffffff);raw=b'';[raw:=raw+b'\\x00'+b'\\xf5\\x6c\\xb8'*32 for _ in range(32)];cd=zlib.compress(raw);idat=b'IDAT'+cd;d+=struct.pack('>I',len(cd))+idat+struct.pack('>I',zlib.crc32(idat)&0xffffffff);iend=b'IEND';d+=struct.pack('>I',0)+iend+struct.pack('>I',zlib.crc32(iend)&0xffffffff);open('crates/app/icons/icon.png','wb').write(d)"`

(If this fails, use any method to create a valid PNG — even copying one from the system.)

- [ ] **Step 6: Create minimal `crates/app/src/main.rs`**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 7: Verify it compiles**

Run: `cargo check -p claude-pending-board-app`
Expected: Compiles (first Tauri build will be slow — downloads WebView2 on Windows)

- [ ] **Step 8: Commit**

```bash
git add crates/app/
git commit -m "feat(app): scaffold Tauri 2 app with single-instance plugin"
```

---

## Task 2: App State and Backend Services

**Files:**
- Create: `crates/app/src/state.rs`
- Create: `crates/app/src/services.rs`
- Modify: `crates/app/src/main.rs`

- [ ] **Step 1: Create `crates/app/src/state.rs`**

```rust
use claude_pending_board_adapters::AdapterRegistry;
use claude_pending_board_core::board::store::StateStore;
use claude_pending_board_core::config::Config;
use claude_pending_board_core::types::Entry;
use claude_pending_board_core::visibility::{VisibilityController, WallClock};
use std::sync::{Arc, Mutex};

/// Shared application state, wrapped in Arc<Mutex<>> for thread safety.
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

    /// Get a sorted snapshot of entries for the UI.
    pub fn entries(&self) -> Vec<Entry> {
        self.store.snapshot()
    }

    /// Get entry count.
    pub fn entry_count(&self) -> usize {
        self.store.len()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2: Create `crates/app/src/services.rs`**

```rust
use crate::state::SharedState;
use claude_pending_board_core::board::compaction;
use claude_pending_board_core::board::watcher::BoardWatcher;
use claude_pending_board_core::reaper::{self, RealProcessTable, RealSessionFiles};
use claude_pending_board_core::visibility::{VisibilityAction, VisibilityEvent};
use chrono::Duration;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;

/// Board file path: ~/.claude/pending/board.jsonl
fn board_path() -> PathBuf {
    let home = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".claude").join("pending").join("board.jsonl")
}

/// Boot all background services.
pub fn boot(app: &AppHandle, state: SharedState) {
    let app_handle = app.clone();
    let state_clone = state.clone();

    // Start board watcher
    let (op_tx, op_rx) = mpsc::unbounded_channel();
    let board_file = board_path();

    // Run startup compaction
    if board_file.exists() {
        if let Err(e) = compaction::compact(&board_file, Duration::hours(24)) {
            tracing::warn!(error = %e, "startup compaction failed");
        }
    }

    match BoardWatcher::start(board_file, op_tx) {
        Ok(_watcher) => {
            tracing::info!("board watcher started");
            // Keep watcher alive by leaking it (it runs until process exit)
            // In a real app you'd store it in state, but for v1 this is fine
            std::mem::forget(_watcher);
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to start board watcher");
        }
    }

    // Spawn the op processing loop
    let app_for_ops = app_handle.clone();
    let state_for_ops = state_clone.clone();
    tauri::async_runtime::spawn(async move {
        process_ops(op_rx, app_for_ops, state_for_ops).await;
    });

    // Spawn the reaper loop
    let app_for_reaper = app_handle.clone();
    let state_for_reaper = state_clone.clone();
    tauri::async_runtime::spawn(async move {
        reaper_loop(app_for_reaper, state_for_reaper).await;
    });

    // Spawn the visibility tick loop
    let app_for_tick = app_handle;
    let state_for_tick = state_clone;
    tauri::async_runtime::spawn(async move {
        visibility_tick_loop(app_for_tick, state_for_tick).await;
    });
}

/// Process ops from the BoardWatcher and update state.
async fn process_ops(
    mut op_rx: mpsc::UnboundedReceiver<Vec<claude_pending_board_core::types::Op>>,
    app: AppHandle,
    state: SharedState,
) {
    while let Some(ops) = op_rx.recv().await {
        let mut s = state.lock().unwrap();
        let count_before = s.store.len();

        for op in ops {
            s.store.apply(op);
        }

        let count_after = s.store.len();
        let entries = s.entries();

        // Drive visibility FSM
        let action = if count_after > count_before {
            s.visibility
                .handle(VisibilityEvent::EntryAdded {
                    board_count: count_after,
                })
        } else if count_after < count_before {
            s.visibility
                .handle(VisibilityEvent::EntryRemoved {
                    board_count: count_after,
                })
        } else {
            VisibilityAction::None
        };

        drop(s); // Release lock before window operations

        // Emit state update to frontend
        let _ = app.emit("entries-updated", &entries);
        let _ = app.emit("badge-count", count_after);

        // Handle visibility action
        apply_visibility_action(&app, &action);
    }
}

/// Run the reaper every 30 seconds.
async fn reaper_loop(app: AppHandle, state: SharedState) {
    let proc_table = RealProcessTable;
    let session_files = RealSessionFiles::new();

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;

        let entries = {
            let s = state.lock().unwrap();
            s.entries()
        };

        let stale_ops = reaper::sweep(&entries, &proc_table, &session_files);

        if !stale_ops.is_empty() {
            // Write stale ops to board file
            let board_file = board_path();
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&board_file)
            {
                use std::io::Write;
                for op in &stale_ops {
                    if let Ok(line) = serde_json::to_string(op) {
                        let _ = writeln!(file, "{}", line);
                    }
                }
            }
            // The BoardWatcher will pick up these writes and process them
        }
    }
}

/// Tick the visibility FSM every 500ms to handle grace and cooldown timers.
async fn visibility_tick_loop(app: AppHandle, state: SharedState) {
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let action = {
            let mut s = state.lock().unwrap();
            s.visibility.handle(VisibilityEvent::Tick)
        };

        apply_visibility_action(&app, &action);
    }
}

/// Apply a visibility action to the HUD window.
fn apply_visibility_action(app: &AppHandle, action: &VisibilityAction) {
    match action {
        VisibilityAction::ShowHud => {
            if let Some(window) = app.get_webview_window("hud") {
                let _ = window.show();
            }
        }
        VisibilityAction::HideHud => {
            if let Some(window) = app.get_webview_window("hud") {
                let _ = window.hide();
            }
        }
        VisibilityAction::UpdateBadge { count } => {
            let _ = app.emit("badge-count", count);
        }
        VisibilityAction::None => {}
    }
}
```

- [ ] **Step 3: Add `dirs-next` to app Cargo.toml**

Add under `[dependencies]`:
```toml
dirs-next = "2"
```

- [ ] **Step 4: Update `crates/app/src/main.rs`**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod services;
mod state;
mod tray;

use state::{AppState, SharedState};
use std::sync::{Arc, Mutex};

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("claude_pending_board=info")
        .init();

    let shared_state: SharedState = Arc::new(Mutex::new(AppState::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .manage(shared_state.clone())
        .setup(move |app| {
            // Boot background services
            services::boot(&app.handle(), shared_state.clone());

            // Set up tray icon
            tray::setup(app)?;

            tracing::info!("Claude Pending Board started");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_entries,
            commands::focus_entry,
            commands::dismiss_hud,
            commands::manual_open,
            commands::get_config,
            commands::apply_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: Create stub `crates/app/src/commands.rs`**

```rust
use crate::state::SharedState;
use claude_pending_board_core::config::Config;
use claude_pending_board_core::types::Entry;
use claude_pending_board_core::visibility::{VisibilityAction, VisibilityEvent};
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
pub fn list_entries(state: State<SharedState>) -> Vec<Entry> {
    let s = state.lock().unwrap();
    s.entries()
}

#[tauri::command]
pub fn focus_entry(
    state: State<SharedState>,
    session_id: String,
) -> Result<String, String> {
    let s = state.lock().unwrap();
    let entry = s
        .store
        .get(&session_id)
        .ok_or_else(|| "entry not found".to_string())?
        .clone();

    // Try to detect and focus the terminal
    if entry.state == claude_pending_board_core::types::EntryState::Live {
        if let Some((adapter, terminal_match)) =
            s.adapter_registry.detect(entry.claude_pid)
        {
            drop(s); // release lock before shelling out
            adapter
                .focus_pane(&terminal_match)
                .map_err(|e| format!("focus failed: {e}"))?;
            return Ok("focused".to_string());
        }
    }

    // Stale or no adapter — spawn resume
    let adapter_name = s.config.default_adapter.clone();
    if let Some(adapter) = s.adapter_registry.get_by_name(&adapter_name) {
        drop(s);
        adapter
            .spawn_resume(&entry.cwd, &entry.session_id)
            .map_err(|e| format!("spawn failed: {e}"))?;
        return Ok("resumed".to_string());
    }

    Err("no adapter available".to_string())
}

#[tauri::command]
pub fn dismiss_hud(
    app: AppHandle,
    state: State<SharedState>,
    reminding_override: Option<bool>,
) -> Result<(), String> {
    let mut s = state.lock().unwrap();
    let action = s.visibility.handle(VisibilityEvent::ManualDismiss {
        reminding_override,
    });
    drop(s);

    if action == VisibilityAction::HideHud {
        if let Some(window) = app.get_webview_window("hud") {
            let _ = window.hide();
        }
    }
    Ok(())
}

#[tauri::command]
pub fn manual_open(app: AppHandle, state: State<SharedState>) -> Result<(), String> {
    let mut s = state.lock().unwrap();
    let action = s.visibility.handle(VisibilityEvent::ManualOpen);
    let entries = s.entries();
    drop(s);

    if action == VisibilityAction::ShowHud {
        if let Some(window) = app.get_webview_window("hud") {
            let _ = window.show();
            let _ = app.emit("entries-updated", &entries);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn get_config(state: State<SharedState>) -> Config {
    let s = state.lock().unwrap();
    s.config.clone()
}

#[tauri::command]
pub fn apply_config(
    state: State<SharedState>,
    config: Config,
) -> Result<(), String> {
    let mut s = state.lock().unwrap();
    s.visibility.update_config(config.clone());
    config
        .save(&Config::default_path())
        .map_err(|e| format!("failed to save config: {e}"))?;
    s.config = config;
    Ok(())
}
```

- [ ] **Step 6: Create stub `crates/app/src/tray.rs`**

```rust
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, Manager,
};

pub fn setup(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let open_item = MenuItemBuilder::with_id("open", "Open").build(app)?;
    let settings_item = MenuItemBuilder::with_id("settings", "Settings...").build(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[&open_item, &settings_item, &quit_item])
        .build()?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "open" => {
                let state: tauri::State<crate::state::SharedState> = app.state();
                let mut s = state.lock().unwrap();
                let action = s.visibility.handle(
                    claude_pending_board_core::visibility::VisibilityEvent::ManualOpen,
                );
                let entries = s.entries();
                drop(s);

                if action == claude_pending_board_core::visibility::VisibilityAction::ShowHud {
                    if let Some(window) = app.get_webview_window("hud") {
                        let _ = window.show();
                        let _ = tauri::Emitter::emit(app, "entries-updated", &entries);
                    }
                }
            }
            "settings" => {
                // Settings window will be created in a later task
                tracing::info!("settings requested");
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                let state: tauri::State<crate::state::SharedState> = app.state();
                let mut s = state.lock().unwrap();
                let action = s.visibility.handle(
                    claude_pending_board_core::visibility::VisibilityEvent::ManualOpen,
                );
                let entries = s.entries();
                drop(s);

                if action == claude_pending_board_core::visibility::VisibilityAction::ShowHud {
                    if let Some(window) = app.get_webview_window("hud") {
                        let _ = window.show();
                        let _ = tauri::Emitter::emit(app, "entries-updated", &entries);
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}
```

- [ ] **Step 7: Verify compilation**

Run: `cargo check -p claude-pending-board-app`
Expected: Compiles (may need to fix import issues — iterate until it compiles)

- [ ] **Step 8: Commit**

```bash
git add crates/app/
git commit -m "feat(app): add AppState, background services, commands, and tray icon"
```

---

## Task 3: HUD Window HTML/CSS

**Files:**
- Replace: `crates/app/ui/hud/index.html`
- Create: `crates/app/ui/hud/style.css`

- [ ] **Step 1: Create `crates/app/ui/hud/style.css`**

Catppuccin Mocha palette. 380x440px HUD with 44px header, entry rows, section dividers.

```css
/* Catppuccin Mocha Palette */
:root {
  --ctp-base: #1e1e2e;
  --ctp-mantle: #181825;
  --ctp-crust: #11111b;
  --ctp-surface0: #313244;
  --ctp-surface1: #45475a;
  --ctp-surface2: #585b70;
  --ctp-overlay0: #6c7086;
  --ctp-overlay1: #7f849c;
  --ctp-text: #cdd6f4;
  --ctp-subtext0: #a6adc8;
  --ctp-subtext1: #bac2de;
  --ctp-red: #f38ba8;
  --ctp-pink: #f5c2e7;
  --ctp-blue: #89b4fa;
  --ctp-lavender: #b4befe;
  --ctp-green: #a6e3a1;
  --ctp-yellow: #f9e2af;
  --ctp-peach: #fab387;
  --ctp-mauve: #cba6f7;
}

* { margin: 0; padding: 0; box-sizing: border-box; }

html, body {
  width: 380px;
  height: 440px;
  overflow: hidden;
  background: var(--ctp-base);
  color: var(--ctp-text);
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  font-size: 13px;
  border-radius: 10px;
  user-select: none;
}

/* Header */
.header {
  height: 44px;
  display: flex;
  align-items: center;
  padding: 0 12px;
  background: var(--ctp-mantle);
  border-bottom: 1px solid var(--ctp-surface0);
  cursor: grab;
  -webkit-app-region: drag;
}

.header:active { cursor: grabbing; }

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--ctp-green);
  margin-right: 8px;
  flex-shrink: 0;
}

.status-dot.has-items { background: var(--ctp-red); }

.title {
  font-weight: 600;
  font-size: 13px;
  flex: 1;
}

.count-badge {
  background: var(--ctp-surface1);
  color: var(--ctp-subtext1);
  font-size: 11px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 10px;
  margin-right: 8px;
}

.header-btn {
  background: none;
  border: none;
  color: var(--ctp-overlay1);
  font-size: 16px;
  cursor: pointer;
  padding: 4px 6px;
  border-radius: 4px;
  -webkit-app-region: no-drag;
}

.header-btn:hover { background: var(--ctp-surface0); color: var(--ctp-text); }

/* Entry list */
.entry-list {
  height: calc(440px - 44px);
  overflow-y: auto;
  padding: 4px 0;
}

.entry-list::-webkit-scrollbar { width: 6px; }
.entry-list::-webkit-scrollbar-track { background: transparent; }
.entry-list::-webkit-scrollbar-thumb { background: var(--ctp-surface1); border-radius: 3px; }

.section-label {
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--ctp-overlay0);
  padding: 8px 12px 4px;
}

.entry-row {
  display: flex;
  align-items: center;
  padding: 8px 12px;
  cursor: pointer;
  border-left: 3px solid transparent;
  transition: background 0.1s;
}

.entry-row:hover { background: var(--ctp-surface0); }
.entry-row.highlight { background: var(--ctp-surface0); transition: background 0.15s; }

.entry-row.permission { border-left-color: var(--ctp-red); }
.entry-row.idle { border-left-color: var(--ctp-blue); }
.entry-row.stale { border-left-color: var(--ctp-overlay0); opacity: 0.7; }

.entry-icon {
  width: 20px;
  font-size: 14px;
  flex-shrink: 0;
  margin-right: 8px;
}

.entry-content { flex: 1; min-width: 0; }

.entry-project {
  font-size: 12px;
  font-weight: 600;
  color: var(--ctp-text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.entry-message {
  font-size: 11px;
  color: var(--ctp-subtext0);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 2px;
}

.entry-time {
  font-size: 10px;
  color: var(--ctp-overlay0);
  flex-shrink: 0;
  margin-left: 8px;
}

/* Empty state */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: calc(440px - 44px);
  color: var(--ctp-overlay0);
}

.empty-state .emoji { font-size: 32px; margin-bottom: 8px; }

/* Dismiss confirmation panel */
.dismiss-panel {
  display: none;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: calc(440px - 44px);
  padding: 24px;
  text-align: center;
}

.dismiss-panel.active { display: flex; }
.dismiss-panel .heading {
  font-size: 15px;
  font-weight: 600;
  color: var(--ctp-text);
  margin-bottom: 4px;
}

.dismiss-panel .subtitle {
  font-size: 12px;
  color: var(--ctp-overlay0);
  margin-bottom: 20px;
}

.dismiss-buttons {
  display: flex;
  gap: 12px;
  width: 100%;
}

.dismiss-btn {
  flex: 1;
  padding: 12px 8px;
  border-radius: 8px;
  border: 2px solid var(--ctp-surface1);
  background: var(--ctp-surface0);
  color: var(--ctp-text);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  text-align: center;
  transition: border-color 0.15s;
}

.dismiss-btn:hover { border-color: var(--ctp-overlay0); }

.dismiss-btn.default {
  border-color: var(--ctp-pink);
}

.dismiss-btn .pill {
  display: inline-block;
  font-size: 9px;
  font-weight: 700;
  text-transform: uppercase;
  background: var(--ctp-pink);
  color: var(--ctp-crust);
  padding: 1px 6px;
  border-radius: 4px;
  margin-left: 6px;
  vertical-align: middle;
}

.dismiss-btn .countdown {
  font-size: 11px;
  color: var(--ctp-subtext0);
}

.dismiss-caption {
  font-size: 11px;
  color: var(--ctp-overlay0);
  margin-top: 6px;
  line-height: 1.4;
}
```

- [ ] **Step 2: Replace `crates/app/ui/hud/index.html`**

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Claude Pending Board</title>
  <link rel="stylesheet" href="style.css">
</head>
<body>
  <!-- Header (always visible) -->
  <div class="header" data-tauri-drag-region>
    <div class="status-dot" id="statusDot"></div>
    <span class="title">Claude Pending</span>
    <span class="count-badge" id="countBadge">0</span>
    <button class="header-btn" id="settingsBtn" title="Settings">&#9881;</button>
    <button class="header-btn" id="dismissBtn" title="Dismiss">&times;</button>
  </div>

  <!-- Entry list (shown when not dismissed) -->
  <div class="entry-list" id="entryList">
    <div class="empty-state" id="emptyState">
      <div class="emoji">&#10003;</div>
      <div>No pending items</div>
    </div>
  </div>

  <!-- Dismiss confirmation panel (hidden by default) -->
  <div class="dismiss-panel" id="dismissPanel">
    <div class="heading" id="dismissHeading">Going silent for 15 minutes</div>
    <div class="subtitle" id="dismissSubtitle">0 items stay on board</div>
    <div class="dismiss-buttons">
      <div>
        <button class="dismiss-btn default" id="btnWakeMe">
          Wake me <span class="pill">DEFAULT</span>
          <div class="countdown" id="wakeMeCountdown">Wake me &middot; 5s</div>
        </button>
        <div class="dismiss-caption">Choose this to wake me<br>after 15 minutes</div>
      </div>
      <div>
        <button class="dismiss-btn" id="btnStaySilent">
          Stay silent
        </button>
        <div class="dismiss-caption">Choose this to stay silent</div>
      </div>
    </div>
  </div>

  <script type="module" src="main.js"></script>
</body>
</html>
```

- [ ] **Step 3: Commit**

```bash
git add crates/app/ui/
git commit -m "feat(app): add HUD window HTML and Catppuccin Mocha CSS"
```

---

## Task 4: HUD Window JavaScript

**Files:**
- Create: `crates/app/ui/hud/main.js`

- [ ] **Step 1: Create `crates/app/ui/hud/main.js`**

```javascript
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;

// DOM elements
const entryList = document.getElementById("entryList");
const emptyState = document.getElementById("emptyState");
const countBadge = document.getElementById("countBadge");
const statusDot = document.getElementById("statusDot");
const dismissBtn = document.getElementById("dismissBtn");
const settingsBtn = document.getElementById("settingsBtn");
const dismissPanel = document.getElementById("dismissPanel");
const dismissHeading = document.getElementById("dismissHeading");
const dismissSubtitle = document.getElementById("dismissSubtitle");
const btnWakeMe = document.getElementById("btnWakeMe");
const btnStaySilent = document.getElementById("btnStaySilent");
const wakeMeCountdown = document.getElementById("wakeMeCountdown");

let currentEntries = [];
let dismissCountdownTimer = null;
let isDismissPanelVisible = false;

// --- Entry rendering ---

function formatTime(ts) {
  const d = new Date(ts);
  const now = new Date();
  const diffMs = now - d;
  const diffMin = Math.floor(diffMs / 60000);
  if (diffMin < 1) return "now";
  if (diffMin < 60) return `${diffMin}m`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h`;
  return `${Math.floor(diffHr / 24)}d`;
}

function extractProjectName(cwd) {
  if (!cwd) return "unknown";
  const parts = cwd.replace(/\\/g, "/").split("/");
  return parts[parts.length - 1] || parts[parts.length - 2] || "unknown";
}

function renderEntries(entries) {
  currentEntries = entries;

  // Remove old entries (keep header and panels)
  const oldEntries = entryList.querySelectorAll(".section-label, .entry-row");
  oldEntries.forEach((el) => el.remove());

  if (entries.length === 0) {
    emptyState.style.display = "flex";
    countBadge.textContent = "0";
    statusDot.classList.remove("has-items");
    return;
  }

  emptyState.style.display = "none";
  countBadge.textContent = entries.length.toString();
  statusDot.classList.add("has-items");

  // Group entries
  const permissions = entries.filter(
    (e) => e.state === "live" && e.notification_type === "permission_prompt"
  );
  const idles = entries.filter(
    (e) => e.state === "live" && e.notification_type === "idle_prompt"
  );
  const stales = entries.filter((e) => e.state === "stale");

  const groups = [
    { label: "PERMISSION", entries: permissions, cls: "permission" },
    { label: "IDLE", entries: idles, cls: "idle" },
    { label: "STALE", entries: stales, cls: "stale" },
  ];

  for (const group of groups) {
    if (group.entries.length === 0) continue;

    const label = document.createElement("div");
    label.className = "section-label";
    label.textContent = group.label;
    entryList.appendChild(label);

    for (const entry of group.entries) {
      const row = document.createElement("div");
      row.className = `entry-row ${group.cls}`;
      row.dataset.sessionId = entry.session_id;

      const icon = group.cls === "permission" ? "🔐" : group.cls === "idle" ? "💬" : "👻";

      row.innerHTML = `
        <span class="entry-icon">${icon}</span>
        <div class="entry-content">
          <div class="entry-project">${extractProjectName(entry.cwd)}</div>
          <div class="entry-message">${escapeHtml(entry.message || "")}</div>
        </div>
        <span class="entry-time">${formatTime(entry.ts)}</span>
      `;

      row.addEventListener("click", () => onEntryClick(entry.session_id));
      entryList.appendChild(row);
    }
  }
}

function escapeHtml(str) {
  const div = document.createElement("div");
  div.textContent = str;
  return div.innerHTML;
}

// --- Actions ---

async function onEntryClick(sessionId) {
  try {
    const result = await invoke("focus_entry", { sessionId });
    console.log("focus result:", result);
  } catch (e) {
    console.error("focus error:", e);
  }
}

// --- Dismiss flow ---

async function showDismissPanel() {
  if (isDismissPanelVisible) return;

  const config = await invoke("get_config");
  isDismissPanelVisible = true;
  entryList.style.display = "none";
  dismissPanel.classList.add("active");

  dismissHeading.textContent = `Going silent for ${config.cooldown_minutes} minutes`;
  dismissSubtitle.textContent = `${currentEntries.length} items stay on board`;

  // Set up default button based on reminding setting
  if (config.reminding_enabled) {
    btnWakeMe.classList.add("default");
    btnStaySilent.classList.remove("default");
    btnWakeMe.querySelector(".pill")?.style.removeProperty("display");
  } else {
    btnStaySilent.classList.add("default");
    btnWakeMe.classList.remove("default");
    btnWakeMe.querySelector(".pill")?.style.setProperty("display", "none");
  }

  // Start countdown
  let remaining = config.dismiss_countdown_secs || 5;
  updateCountdown(remaining, config.reminding_enabled);

  dismissCountdownTimer = setInterval(() => {
    remaining--;
    updateCountdown(remaining, config.reminding_enabled);
    if (remaining <= 0) {
      clearInterval(dismissCountdownTimer);
      commitDismiss(null); // null = use global default
    }
  }, 1000);
}

function updateCountdown(secs, isWakeMeDefault) {
  if (isWakeMeDefault) {
    wakeMeCountdown.textContent = `Wake me \u00B7 ${secs}s`;
  } else {
    wakeMeCountdown.textContent = "";
  }
}

function hideDismissPanel() {
  isDismissPanelVisible = false;
  if (dismissCountdownTimer) {
    clearInterval(dismissCountdownTimer);
    dismissCountdownTimer = null;
  }
  dismissPanel.classList.remove("active");
  entryList.style.display = "block";
}

async function commitDismiss(remindingOverride) {
  hideDismissPanel();
  try {
    await invoke("dismiss_hud", { remindingOverride });
  } catch (e) {
    console.error("dismiss error:", e);
  }
}

// --- Event listeners ---

dismissBtn.addEventListener("click", (e) => {
  e.stopPropagation();
  showDismissPanel();
});

btnWakeMe.addEventListener("click", () => commitDismiss(true));
btnStaySilent.addEventListener("click", () => commitDismiss(false));

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && isDismissPanelVisible) {
    commitDismiss(null); // default
  }
});

settingsBtn.addEventListener("click", async () => {
  // TODO: open settings window (Phase 3, Task 7)
  console.log("settings clicked");
});

// --- Backend event listeners ---

listen("entries-updated", (event) => {
  renderEntries(event.payload);
});

listen("badge-count", (event) => {
  countBadge.textContent = event.payload.toString();
});

// --- Initial load ---

(async () => {
  try {
    const entries = await invoke("list_entries");
    renderEntries(entries);
  } catch (e) {
    console.error("initial load error:", e);
  }
})();
```

- [ ] **Step 2: Commit**

```bash
git add crates/app/ui/hud/main.js
git commit -m "feat(app): add HUD JavaScript with entry rendering, dismiss flow, and event listeners"
```

---

## Task 5: HUD Window Creation in Tauri

**Files:**
- Modify: `crates/app/src/main.rs`

The HUD window needs to be created programmatically (not via tauri.conf.json) because we need platform-specific non-activating flags.

- [ ] **Step 1: Update `main.rs` to create HUD window in setup**

Add to the `setup` closure, after `tray::setup(app)?;`:

```rust
// Create HUD window (hidden initially)
use tauri::WebviewUrl;
let hud_window = tauri::WebviewWindowBuilder::new(
    app,
    "hud",
    WebviewUrl::App("index.html".into()),
)
    .title("Claude Pending Board")
    .inner_size(380.0, 440.0)
    .resizable(false)
    .decorations(false)
    .transparent(false)
    .always_on_top(true)
    .visible(false)
    .skip_taskbar(true)
    .build()?;

// Platform-specific: attempt to set non-activating behavior
#[cfg(target_os = "windows")]
{
    use tauri::Manager;
    // On Windows, always_on_top + skip_taskbar is the best we can do
    // WS_EX_NOACTIVATE requires raw HWND manipulation via windows-rs
    tracing::info!("HUD window created (Windows)");
}
```

- [ ] **Step 2: Verify the app compiles**

Run: `cargo check -p claude-pending-board-app`
Expected: Compiles

- [ ] **Step 3: Commit**

```bash
git add crates/app/src/main.rs
git commit -m "feat(app): create HUD window programmatically with always-on-top and skip-taskbar"
```

---

## Task 6: Settings Window

**Files:**
- Create: `crates/app/ui/settings/index.html`
- Create: `crates/app/ui/settings/style.css`
- Create: `crates/app/ui/settings/main.js`
- Modify: `crates/app/src/tray.rs` (open settings window)

- [ ] **Step 1: Create `crates/app/ui/settings/index.html`**

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Settings — Claude Pending Board</title>
  <link rel="stylesheet" href="style.css">
</head>
<body>
  <h1>Settings</h1>

  <div class="setting-group">
    <label>Cooldown after dismiss (minutes)</label>
    <input type="range" id="cooldownMinutes" min="1" max="120" value="15">
    <span class="range-value" id="cooldownValue">15</span>
  </div>

  <div class="setting-group">
    <label>Reminding enabled</label>
    <input type="checkbox" id="remindingEnabled" checked>
  </div>

  <div class="setting-group">
    <label>Auto-hide grace delay (seconds)</label>
    <input type="range" id="autoHideGrace" min="0" max="10" value="2">
    <span class="range-value" id="graceValue">2</span>
  </div>

  <div class="setting-group">
    <label>Dismiss countdown (seconds)</label>
    <input type="range" id="dismissCountdown" min="2" max="10" value="5">
    <span class="range-value" id="countdownValue">5</span>
  </div>

  <div class="setting-group">
    <label>Skip dismiss confirmation</label>
    <input type="checkbox" id="skipConfirmation">
  </div>

  <div class="setting-group">
    <label>Default terminal adapter</label>
    <select id="defaultAdapter">
      <option value="wezterm">WezTerm</option>
      <option value="iterm2">iTerm2</option>
    </select>
  </div>

  <div class="setting-group">
    <label>Debug logging</label>
    <input type="checkbox" id="debugLogging">
  </div>

  <div class="actions">
    <button id="saveBtn">Save</button>
    <button id="resetPositionBtn">Reset HUD Position</button>
  </div>

  <div class="status" id="statusMsg"></div>

  <script type="module" src="main.js"></script>
</body>
</html>
```

- [ ] **Step 2: Create `crates/app/ui/settings/style.css`**

```css
:root {
  --ctp-base: #1e1e2e;
  --ctp-mantle: #181825;
  --ctp-surface0: #313244;
  --ctp-surface1: #45475a;
  --ctp-text: #cdd6f4;
  --ctp-subtext0: #a6adc8;
  --ctp-green: #a6e3a1;
  --ctp-pink: #f5c2e7;
}

* { margin: 0; padding: 0; box-sizing: border-box; }

body {
  background: var(--ctp-base);
  color: var(--ctp-text);
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  font-size: 14px;
  padding: 24px;
}

h1 {
  font-size: 18px;
  margin-bottom: 20px;
  color: var(--ctp-pink);
}

.setting-group {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 0;
  border-bottom: 1px solid var(--ctp-surface0);
}

.setting-group label { flex: 1; }

.setting-group input[type="range"] { width: 120px; }
.setting-group input[type="checkbox"] { width: 20px; height: 20px; }
.setting-group select {
  background: var(--ctp-surface0);
  color: var(--ctp-text);
  border: 1px solid var(--ctp-surface1);
  padding: 4px 8px;
  border-radius: 4px;
}

.range-value {
  width: 30px;
  text-align: right;
  color: var(--ctp-subtext0);
  margin-left: 8px;
}

.actions {
  margin-top: 20px;
  display: flex;
  gap: 12px;
}

.actions button {
  padding: 8px 16px;
  border-radius: 6px;
  border: none;
  cursor: pointer;
  font-size: 13px;
  font-weight: 600;
}

#saveBtn {
  background: var(--ctp-green);
  color: var(--ctp-base);
}

#resetPositionBtn {
  background: var(--ctp-surface1);
  color: var(--ctp-text);
}

.status {
  margin-top: 12px;
  font-size: 12px;
  color: var(--ctp-green);
  min-height: 20px;
}
```

- [ ] **Step 3: Create `crates/app/ui/settings/main.js`**

```javascript
const { invoke } = window.__TAURI__.core;

const cooldownSlider = document.getElementById("cooldownMinutes");
const cooldownValue = document.getElementById("cooldownValue");
const remindingCheckbox = document.getElementById("remindingEnabled");
const graceSlider = document.getElementById("autoHideGrace");
const graceValue = document.getElementById("graceValue");
const countdownSlider = document.getElementById("dismissCountdown");
const countdownValue = document.getElementById("countdownValue");
const skipConfirmation = document.getElementById("skipConfirmation");
const defaultAdapter = document.getElementById("defaultAdapter");
const debugLogging = document.getElementById("debugLogging");
const saveBtn = document.getElementById("saveBtn");
const resetPositionBtn = document.getElementById("resetPositionBtn");
const statusMsg = document.getElementById("statusMsg");

// Sync slider display values
cooldownSlider.addEventListener("input", () => {
  cooldownValue.textContent = cooldownSlider.value;
});
graceSlider.addEventListener("input", () => {
  graceValue.textContent = graceSlider.value;
});
countdownSlider.addEventListener("input", () => {
  countdownValue.textContent = countdownSlider.value;
});

// Load current config
async function loadConfig() {
  try {
    const config = await invoke("get_config");
    cooldownSlider.value = config.cooldown_minutes;
    cooldownValue.textContent = config.cooldown_minutes;
    remindingCheckbox.checked = config.reminding_enabled;
    graceSlider.value = config.auto_hide_grace_secs;
    graceValue.textContent = config.auto_hide_grace_secs;
    countdownSlider.value = config.dismiss_countdown_secs;
    countdownValue.textContent = config.dismiss_countdown_secs;
    skipConfirmation.checked = config.skip_dismiss_confirmation;
    defaultAdapter.value = config.default_adapter;
    debugLogging.checked = config.debug_logging;
  } catch (e) {
    statusMsg.textContent = "Failed to load config: " + e;
    statusMsg.style.color = "#f38ba8";
  }
}

// Save config
saveBtn.addEventListener("click", async () => {
  const config = {
    cooldown_minutes: parseInt(cooldownSlider.value),
    reminding_enabled: remindingCheckbox.checked,
    auto_hide_grace_secs: parseInt(graceSlider.value),
    dismiss_countdown_secs: parseInt(countdownSlider.value),
    skip_dismiss_confirmation: skipConfirmation.checked,
    default_adapter: defaultAdapter.value,
    hud_position: null,
    debug_logging: debugLogging.checked,
  };

  try {
    await invoke("apply_config", { config });
    statusMsg.textContent = "Settings saved";
    statusMsg.style.color = "#a6e3a1";
    setTimeout(() => { statusMsg.textContent = ""; }, 2000);
  } catch (e) {
    statusMsg.textContent = "Failed to save: " + e;
    statusMsg.style.color = "#f38ba8";
  }
});

// Reset HUD position
resetPositionBtn.addEventListener("click", async () => {
  const config = await invoke("get_config");
  config.hud_position = null;
  try {
    await invoke("apply_config", { config });
    statusMsg.textContent = "HUD position reset";
    statusMsg.style.color = "#a6e3a1";
    setTimeout(() => { statusMsg.textContent = ""; }, 2000);
  } catch (e) {
    statusMsg.textContent = "Failed: " + e;
    statusMsg.style.color = "#f38ba8";
  }
});

// Initial load
loadConfig();
```

- [ ] **Step 4: Update tray.rs to open settings window**

In `crates/app/src/tray.rs`, replace the `"settings"` match arm:

```rust
"settings" => {
    // Create settings window if it doesn't exist, or show it
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
    } else {
        let _ = tauri::WebviewWindowBuilder::new(
            app,
            "settings",
            tauri::WebviewUrl::App("../settings/index.html".into()),
        )
            .title("Settings — Claude Pending Board")
            .inner_size(480.0, 500.0)
            .resizable(true)
            .build();
    }
}
```

- [ ] **Step 5: Verify compilation**

Run: `cargo check -p claude-pending-board-app`
Expected: Compiles

- [ ] **Step 6: Commit**

```bash
git add crates/app/ui/settings/ crates/app/src/tray.rs
git commit -m "feat(app): add Settings window with config form and live reload"
```

---

## Task 7: Final Integration Verification

- [ ] **Step 1: Run all workspace tests**

Run: `cargo test --workspace`
Expected: All tests pass (55 core + 7 adapters = 62)

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: 0 warnings

- [ ] **Step 3: Run fmt**

Run: `cargo fmt --check --all`
Expected: Clean (fix with `cargo fmt --all` if needed)

- [ ] **Step 4: Try `cargo tauri dev` smoke test**

Run: `cargo tauri dev`
Expected: The app should launch, show a tray icon, and the HUD should be hidden initially. If there are runtime errors, fix them.

Note: This requires Tauri CLI (`cargo install tauri-cli`). If not available, `cargo run -p claude-pending-board-app` should at least start the process.

- [ ] **Step 5: Commit any fixes and tag**

```bash
git add -A
git commit -m "chore: phase 3 complete — Tauri app with HUD, dismiss panel, and settings"
git tag v0.0.3-app
```

---

## Self-Review Checklist

1. **Spec coverage:**
   - Floating HUD window (380x440, non-resizable, draggable, drop shadow): Task 3 HTML/CSS + Task 5 window creation — covered
   - Header (status dot, title, count badge, gear, dismiss): Task 3 HTML — covered
   - List scrolls: Task 3 CSS `overflow-y: auto` — covered
   - Non-activating window: Task 5 `always_on_top` + `skip_taskbar` — covered (full WS_EX_NOACTIVATE is a future enhancement)
   - Auto show/hide: Task 2 services.rs `process_ops` + visibility FSM — covered
   - Grace timer: Task 2 `visibility_tick_loop` — covered
   - Manual dismiss with cooldown: Task 4 JS dismiss flow + Task 2 commands — covered
   - Dismiss confirmation panel: Task 3 HTML + Task 4 JS — covered
   - 5-second countdown with default: Task 4 JS — covered
   - Esc applies default: Task 4 JS keydown listener — covered
   - Per-dismiss override: Task 4 JS `commitDismiss(true/false)` — covered
   - Helper captions: Task 3 HTML — covered
   - Click to focus live: Task 2 `focus_entry` command — covered
   - Click to resume stale: Task 2 `focus_entry` command fallback — covered
   - Settings surface: Task 6 — covered
   - Settings fields (cooldown, reminding, grace, countdown, skip-confirm, adapter, reset position): Task 6 — covered
   - Live config reload: Task 2 `apply_config` command — covered
   - Tray icon with context menu: Task 2 tray.rs — covered

2. **Placeholder scan:** No TBD/TODO found except the settings button comment in main.js which is addressed by Task 6.

3. **Type consistency:** `Entry`, `Config`, `VisibilityEvent`, `VisibilityAction` — all match core crate definitions. Command names match between `generate_handler!` and JS `invoke()` calls.

---

## Smoke Test Results (2026-04-17)

Ran `scripts/smoke-test.ps1` against the built app. Initial run failed step 1 (HUD did not list the entry). Four root causes fixed:

1. **No Tauri 2 capability file.** Tauri 2 blocks all IPC for webviews that don't match a capability, so `invoke()` and `listen()` in the HUD silently failed — the HUD rendered but never received entries. Added `crates/app/capabilities/default.json` granting `core:default` to the `hud` and `settings` windows.

2. **Startup race between watcher and HUD window.** `services::boot()` was called before the HUD window was built. When `board.jsonl` already contained entries at startup, the watcher's synchronous initial read pushed ops into the channel, and the spawned `process_ops` could try `window.show()` before the window existed — `get_webview_window("hud")` returned `None` and the show action was silently dropped. Reordered `main.rs::setup` to build the HUD window *before* booting services.

3. **Settings window loaded the HUD UI.** `tray.rs` used `WebviewUrl::App("../settings/index.html")`, but Tauri's asset resolver disallows `..` escaping the `frontendDist` root. Restructured: `frontendDist` is now `ui` (not `ui/hud`), HUD URL is `hud/index.html`, Settings URL is `settings/index.html`.

4. **HUD gear button was a stub.** The settings button handler in `ui/hud/main.js` only called `console.log`. Added a new `open_settings` Tauri command that gets-or-creates the settings window, registered it in `invoke_handler`, and wired the gear button to call it.

Post-fix verification (fresh start, empty board, step 1 written): HUD becomes visible at default position with Catppuccin-dark theme, `PERMISSION` section showing the lock-icon entry with project name, message, and `now` timestamp. No startup ordering issues across restarts with persisted entries.
