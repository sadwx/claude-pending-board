# Phase 4: Plugin, Docs, CI, and Release Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship v0.1.0 — package the pending-board as a Claude Code plugin, finalize end-user documentation, set up CI/CD on GitHub, and cut a signed release.

**Architecture:** The plugin is a thin wrapper around the existing `scripts/pending_hook.ps1` and `pending_hook.sh` — it registers the three Claude Code hooks (`Notification`, `UserPromptSubmit`, `Stop`) and ships a `/pending-board` slash command with `status`, `install`, `doctor`, and `hooks-uninstall` subcommands. CI runs `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` on every push; a matrix build job produces platform-specific Tauri binaries (Windows `.msi`/`.exe`, macOS `.dmg`, Linux `.AppImage`/`.deb`). The release workflow triggers on version tag push, uploads signed artifacts to GitHub Releases, and drafts release notes.

**Tech Stack:** Claude Code plugin manifest (`.claude-plugin/plugin.json`), GitHub Actions, `tauri-action` for matrix builds, `cross-rs` for cross-compilation where needed.

---

## File Structure

```
claude-pending-board/
├── plugin/
│   ├── .claude-plugin/
│   │   └── plugin.json              # Plugin manifest (hooks + slash commands)
│   ├── hooks/
│   │   ├── pending_hook.ps1         # copied from scripts/ on build
│   │   └── pending_hook.sh          # copied from scripts/ on build
│   ├── commands/
│   │   └── pending-board.md         # /pending-board slash command
│   └── README.md                    # Plugin usage docs
├── docs/
│   ├── release-checklist.md         # Manual UX checklist from design
│   └── screenshots/                 # existing
├── .github/
│   └── workflows/
│       ├── ci.yml                   # fmt + clippy + test on push/PR
│       ├── release.yml              # matrix build + GitHub Release on tag
│       └── adapter-contract.yml     # ignored tests w/ real WezTerm on Linux
├── tests/
│   └── end_to_end.rs                # workspace-level integration tests
├── README.md                        # existing — may need refinements
└── INSTALL.md                       # existing — may need refinements
```

---

## Task 1: Claude Code Plugin Skeleton

**Files:**
- Create: `plugin/.claude-plugin/plugin.json`
- Create: `plugin/hooks/pending_hook.ps1` (copy from `scripts/`)
- Create: `plugin/hooks/pending_hook.sh` (copy from `scripts/`)
- Create: `plugin/README.md`

- [ ] **Step 1: Create `plugin/.claude-plugin/plugin.json`**

```json
{
  "$schema": "https://json.schemastore.org/claude-code-plugin.json",
  "name": "claude-pending-board",
  "version": "0.1.0",
  "description": "Surface every pending Claude Code session across all projects in a single floating HUD window.",
  "author": {
    "name": "sadwx",
    "url": "https://github.com/sadwx"
  },
  "homepage": "https://github.com/sadwx/claude-pending-board",
  "repository": "https://github.com/sadwx/claude-pending-board",
  "hooks": {
    "Notification": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "pwsh -NoProfile -ExecutionPolicy Bypass -File ${CLAUDE_PLUGIN_ROOT}/hooks/pending_hook.ps1",
            "platform": "windows"
          },
          {
            "type": "command",
            "command": "bash ${CLAUDE_PLUGIN_ROOT}/hooks/pending_hook.sh",
            "platform": "darwin"
          },
          {
            "type": "command",
            "command": "bash ${CLAUDE_PLUGIN_ROOT}/hooks/pending_hook.sh",
            "platform": "linux"
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "pwsh -NoProfile -ExecutionPolicy Bypass -File ${CLAUDE_PLUGIN_ROOT}/hooks/pending_hook.ps1",
            "platform": "windows"
          },
          {
            "type": "command",
            "command": "bash ${CLAUDE_PLUGIN_ROOT}/hooks/pending_hook.sh",
            "platform": "darwin"
          },
          {
            "type": "command",
            "command": "bash ${CLAUDE_PLUGIN_ROOT}/hooks/pending_hook.sh",
            "platform": "linux"
          }
        ]
      }
    ],
    "Stop": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "pwsh -NoProfile -ExecutionPolicy Bypass -File ${CLAUDE_PLUGIN_ROOT}/hooks/pending_hook.ps1",
            "platform": "windows"
          },
          {
            "type": "command",
            "command": "bash ${CLAUDE_PLUGIN_ROOT}/hooks/pending_hook.sh",
            "platform": "darwin"
          },
          {
            "type": "command",
            "command": "bash ${CLAUDE_PLUGIN_ROOT}/hooks/pending_hook.sh",
            "platform": "linux"
          }
        ]
      }
    ]
  }
}
```

- [ ] **Step 2: Copy hook scripts into plugin directory**

Run (from repo root):
```bash
mkdir -p plugin/hooks
cp scripts/pending_hook.ps1 plugin/hooks/
cp scripts/pending_hook.sh plugin/hooks/
chmod +x plugin/hooks/pending_hook.sh
```

- [ ] **Step 3: Create `plugin/README.md`**

```markdown
# Claude Pending Board — Claude Code Plugin

A thin wrapper that registers Claude Code hooks so pending prompts land on the Claude Pending Board HUD.

## Requirements

You must also have the [Claude Pending Board tray app](https://github.com/sadwx/claude-pending-board/releases) installed and running. The plugin writes entries to `~/.claude/pending/board.jsonl`; the tray app reads them and surfaces the HUD.

## Install

```bash
/plugin marketplace add github:sadwx/claude-pending-board
/plugin install claude-pending-board@claude-pending-board
/reload-plugins
```

## Verify

Run:
```bash
/pending-board doctor
```

This checks:
- ✓ The three hooks are registered
- ✓ The hook scripts exist and are executable
- ✓ `~/.claude/pending/board.jsonl` is writable
- ✓ The configured terminal adapter binary is in `PATH`

## How it works

When Claude Code fires a `Notification`, `UserPromptSubmit`, or `Stop` event, the plugin's hook scripts append a JSONL line to `~/.claude/pending/board.jsonl`. The Claude Pending Board tray app watches this file, renders entries in its HUD, and lets you click through to the owning terminal pane.

## Uninstall

```bash
/plugin uninstall claude-pending-board
```

The hook scripts stop being invoked. Your existing `~/.claude/pending/` state (config, logs) is preserved — delete that directory manually if you want a clean slate.
```

- [ ] **Step 4: Commit**

```bash
git add plugin/.claude-plugin plugin/hooks plugin/README.md
git commit -m "feat(plugin): scaffold Claude Code plugin with hooks and README"
```

---

## Task 2: `/pending-board` Slash Command

**Files:**
- Create: `plugin/commands/pending-board.md`

- [ ] **Step 1: Create `plugin/commands/pending-board.md`**

```markdown
---
description: Manage the Claude Pending Board plugin — status, install, doctor, uninstall hooks
argument-hint: "[status | install | doctor | hooks-uninstall]"
---

# /pending-board

You are the claude-pending-board operator. Run the requested subcommand and report the result to the user.

Subcommand: **$ARGUMENTS** (defaults to `status` if empty)

---

## `status`

Report the current state of the pending board:

1. Read `~/.claude/pending/board.jsonl` (if it exists). Count lines.
2. Parse it and count live entries by notification type (permission_prompt, idle_prompt) and state (live, stale).
3. Report the tray app binary location if it's known (check `~/.claude/pending/config.toml` or PATH for `claude-pending-board-app`).
4. Report whether the app process is currently running (via `tasklist` on Windows or `pgrep` on Unix).

Output format:
```
Claude Pending Board — status
  board.jsonl: <N> lines, <M> live entries (<P> permission, <I> idle), <S> stale
  tray app: <running | not running> (<path if known>)
  last activity: <timestamp of most recent op in board.jsonl, or "never">
```

---

## `install`

Explain to the user how to install the tray app:

1. Visit https://github.com/sadwx/claude-pending-board/releases
2. Download the artifact for your OS
3. Launch it — the tray icon should appear

Do NOT download or run the binary for them. Just print the instructions.

---

## `doctor`

Run the following diagnostic checks and report each as ✓ or ✗ with a remediation hint on failure:

1. **Hooks registered**: Read `~/.claude/settings.json` OR verify the plugin is enabled in `~/.claude/plugins/installed_plugins.json`. Look for `Notification`, `UserPromptSubmit`, and `Stop` hooks.
2. **Hook scripts exist**: Check `${CLAUDE_PLUGIN_ROOT}/hooks/pending_hook.ps1` (Windows) or `pending_hook.sh` (Unix).
3. **Board file writable**: Try `touch ~/.claude/pending/board.jsonl` (create if missing). Verify append works.
4. **Log directory writable**: Same for `~/.claude/pending/logs/`.
5. **Terminal adapter in PATH**: Check `wezterm --version` (Windows/Linux) or `osascript -e 'tell application "iTerm2" to version'` (macOS).
6. **Tray app installed**: Check for `claude-pending-board-app` on PATH or standard install locations.

Output format:
```
Claude Pending Board — doctor
  ✓ Hooks registered: Notification, UserPromptSubmit, Stop
  ✓ Hook scripts: pending_hook.ps1 (at <path>)
  ✓ Board file writable: ~/.claude/pending/board.jsonl
  ✓ Log directory writable: ~/.claude/pending/logs/
  ✓ Terminal adapter: wezterm 20240203-...
  ✗ Tray app: not found — install from https://github.com/sadwx/claude-pending-board/releases
```

---

## `hooks-uninstall`

Help the user uninstall the plugin:

1. Explain that `/plugin uninstall claude-pending-board` removes the hooks.
2. Offer to clean up `~/.claude/pending/` (board.jsonl, logs, config) — but do NOT delete without confirmation.

---

**Default (no argument):** Run `status`.
```

- [ ] **Step 2: Commit**

```bash
git add plugin/commands/pending-board.md
git commit -m "feat(plugin): add /pending-board slash command (status, install, doctor, uninstall)"
```

---

## Task 3: README Refinements

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Read existing README**

Run: `cat README.md`

- [ ] **Step 2: Ensure README has these sections**

If any are missing, add them:

1. **Hero screenshot** — embed `docs/screenshots/current-dismiss-v2.png` or create a composite showing HUD + settings
2. **Badges** — GitHub release, license, platform support (add after title)
3. **Features** — 5-7 bullets from the spec (unified inbox, non-activating HUD, click-to-focus, live/stale, dismiss with cooldown, etc.)
4. **How it works** — ASCII diagram of hook → board.jsonl → watcher → HUD → terminal adapter
5. **Quick start** — minimal install + first run
6. **Status** — link to milestones, current version
7. **Contributing** — link to OpenSpec + CONTRIBUTING.md if exists

- [ ] **Step 3: Verify the screenshot reference works**

View the rendered README (on GitHub after push) and confirm the image displays.

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: refine README with features, architecture diagram, and screenshot"
```

---

## Task 4: Release Checklist

**Files:**
- Create: `docs/release-checklist.md`

- [ ] **Step 1: Create `docs/release-checklist.md`**

```markdown
# Release Checklist

Manual verification required before cutting a new release.

## Pre-release

- [ ] `cargo fmt --check --all` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo test --workspace` all green (expected: ~60+ tests)
- [ ] `cargo tauri build --release` succeeds on each platform (Win / macOS / Linux)

## Smoke tests — per platform

Install the release artifact. Then run through:

- [ ] Tray icon appears on app launch
- [ ] HUD is hidden at startup (no entries)
- [ ] `echo '{"op":"add",...}' >> ~/.claude/pending/board.jsonl` causes HUD to auto-appear within 1 second
- [ ] HUD shows the entry under the right section (PERMISSION / IDLE)
- [ ] Clicking an entry focuses the owning terminal (requires WezTerm running)
- [ ] Clicking a stale entry spawns `claude --resume <id>` in a new tab
- [ ] HUD auto-hides 2s after board empties
- [ ] Dismiss X opens confirmation panel with DEFAULT pill and countdown
- [ ] Esc / countdown expiry commits the default
- [ ] Click "Wake me" / "Stay silent" commits immediately with override
- [ ] Tray left-click re-opens HUD from cooldown
- [ ] Tray right-click → Settings opens the settings window
- [ ] Settings form loads current config, Save persists, window auto-hides
- [ ] Reset HUD Position moves the HUD to bottom-right of primary monitor
- [ ] Non-activating: opening HUD does NOT steal keyboard focus from current app

## Plugin tests

- [ ] `/plugin install claude-pending-board` from marketplace succeeds
- [ ] `/pending-board doctor` passes all checks with tray app + WezTerm installed
- [ ] Real Claude Code session with permission prompt writes to board.jsonl within 100ms

## Multi-monitor

- [ ] Drag HUD to secondary monitor, close app, relaunch — HUD restores on secondary
- [ ] Unplug secondary monitor while HUD was there — next launch falls back to primary

## Edge cases

- [ ] Kill the tray app while HUD is open — no zombie process, clean exit
- [ ] 50 entries in board.jsonl — HUD scrolls, doesn't grow
- [ ] Malformed line in board.jsonl — skipped silently, warning in app.log
- [ ] Delete board.jsonl while app is running — in-memory state clears, no crash

## Release

- [ ] Bump `version` in workspace `Cargo.toml`, `plugin/.claude-plugin/plugin.json`, `crates/app/tauri.conf.json`
- [ ] Update `CHANGELOG.md` (or release notes inline in the tag)
- [ ] Tag the release: `git tag -a v0.1.0 -m "..."`
- [ ] Push tag: `git push origin v0.1.0` — triggers `.github/workflows/release.yml`
- [ ] Verify GitHub Release has artifacts for all three OSes
- [ ] Verify `/plugin install` pulls the new version
- [ ] Announce (README badge, Discord, Twitter, etc.)
```

- [ ] **Step 2: Commit**

```bash
git add docs/release-checklist.md
git commit -m "docs: add release checklist for manual verification"
```

---

## Task 5: CI Workflow — fmt + clippy + test

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create `.github/workflows/ci.yml`**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  fmt:
    name: cargo fmt --check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --check --all

  clippy:
    name: cargo clippy
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - name: Install Tauri system deps (Linux)
        if: matrix.os == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --workspace --all-targets -- -D warnings

  test:
    name: cargo test
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install Tauri system deps (Linux)
        if: matrix.os == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace --no-fail-fast
```

- [ ] **Step 2: Commit and push**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add fmt + clippy + test workflow on push/PR"
git push origin main
```

- [ ] **Step 3: Verify the workflow runs**

Check `https://github.com/sadwx/claude-pending-board/actions` — the workflow should trigger and complete green.

If any job fails, iterate on the workflow locally (using `act` if available) or fix the code until it passes.

---

## Task 6: Release Workflow — matrix build + GitHub Release

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Create `.github/workflows/release.yml`**

```yaml
name: Release

on:
  push:
    tags:
      - "v*"

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: macos-latest
            target: universal-apple-darwin
          - platform: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - platform: windows-latest
            target: x86_64-pc-windows-msvc

    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install Tauri system deps (Linux)
        if: matrix.platform == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

      - uses: Swatinem/rust-cache@v2

      - uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          projectPath: crates/app
          tagName: ${{ github.ref_name }}
          releaseName: "Claude Pending Board ${{ github.ref_name }}"
          releaseBody: "See [release-checklist](./docs/release-checklist.md) and the changelog below."
          releaseDraft: true
          prerelease: false
          args: ${{ matrix.platform == 'macos-latest' && '--target universal-apple-darwin' || '' }}
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add release workflow building Tauri artifacts on tag push"
```

- [ ] **Step 3: Dry-run verification**

Do NOT tag a release yet. Just push the workflow and let it be available. Actual release happens in Task 9.

```bash
git push origin main
```

Check that the workflow is visible in Actions but not running.

---

## Task 7: End-to-End Integration Tests

**Files:**
- Create: `tests/end_to_end.rs` (workspace root)
- Modify: `Cargo.toml` (add `tests` to workspace members if needed)

- [ ] **Step 1: Create `tests/end_to_end.rs` at the workspace root**

```rust
//! End-to-end integration tests that exercise core pieces together.
//!
//! These tests don't boot the Tauri app; they verify the data flow:
//! write to board.jsonl → parser → store → compaction round-trip.

use claude_pending_board_core::board::{compaction, parser, store::StateStore};
use claude_pending_board_core::types::{EntryState, NotificationType, Op};
use chrono::{Duration, Utc};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn add_line(session_id: &str, ts: &str, kind: &str) -> String {
    format!(
        r#"{{"op":"add","ts":"{ts}","session_id":"{session_id}","cwd":"/tmp","claude_pid":1,"terminal_pid":null,"transcript_path":"/tmp/t","notification_type":"{kind}","message":"m"}}"#
    )
}

fn clear_line(session_id: &str, ts: &str) -> String {
    format!(r#"{{"op":"clear","ts":"{ts}","session_id":"{session_id}","reason":"user_replied"}}"#)
}

fn stale_line(session_id: &str, ts: &str) -> String {
    format!(r#"{{"op":"stale","ts":"{ts}","session_id":"{session_id}","reason":"pid_dead"}}"#)
}

#[test]
fn golden_path_replay_and_snapshot_order() {
    let text = [
        add_line("a", "2026-04-17T10:00:00Z", "permission_prompt"),
        add_line("b", "2026-04-17T10:01:00Z", "idle_prompt"),
        add_line("c", "2026-04-17T10:02:00Z", "permission_prompt"),
        clear_line("a", "2026-04-17T10:03:00Z"),
    ]
    .join("\n");

    let (ops, skipped) = parser::parse_lines(&text);
    assert_eq!(ops.len(), 4);
    assert_eq!(skipped, 0);

    let mut store = StateStore::new();
    store.apply_all(ops);

    let snapshot = store.snapshot();
    assert_eq!(snapshot.len(), 2);
    // c (permission, ts 10:02) before b (idle, ts 10:01)
    assert_eq!(snapshot[0].session_id, "c");
    assert_eq!(snapshot[1].session_id, "b");
}

#[test]
fn compaction_roundtrip_preserves_current_state() {
    let dir = TempDir::new().unwrap();
    let path: PathBuf = dir.path().join("board.jsonl");

    let text = [
        add_line("a", "2026-04-17T10:00:00Z", "permission_prompt"),
        add_line("b", "2026-04-17T10:01:00Z", "idle_prompt"),
        clear_line("a", "2026-04-17T10:03:00Z"),
        add_line("d", "2026-04-17T10:04:00Z", "permission_prompt"),
    ]
    .join("\n")
        + "\n";

    fs::write(&path, text).unwrap();

    let result = compaction::compact(&path, Duration::hours(24)).unwrap();
    assert_eq!(result.entries_before, 4);
    assert_eq!(result.entries_after, 2);

    // Reload and verify
    let content = fs::read_to_string(&path).unwrap();
    let (ops, _) = parser::parse_lines(&content);
    let mut store = StateStore::new();
    store.apply_all(ops);
    let snapshot = store.snapshot();
    let ids: Vec<&str> = snapshot.iter().map(|e| e.session_id.as_str()).collect();
    assert!(ids.contains(&"b"));
    assert!(ids.contains(&"d"));
    assert!(!ids.contains(&"a"));
}

#[test]
fn expired_stale_entries_dropped_during_compaction() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("board.jsonl");

    let old = (Utc::now() - Duration::hours(48)).to_rfc3339();
    let fresh = (Utc::now() - Duration::hours(1)).to_rfc3339();
    let stale_old = (Utc::now() - Duration::hours(25)).to_rfc3339();
    let stale_fresh = Utc::now().to_rfc3339();

    let text = [
        add_line("expired", &old, "permission_prompt"),
        stale_line("expired", &stale_old),
        add_line("recent", &fresh, "permission_prompt"),
        stale_line("recent", &stale_fresh),
    ]
    .join("\n")
        + "\n";

    fs::write(&path, text).unwrap();

    compaction::compact(&path, Duration::hours(24)).unwrap();

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("recent"));
    assert!(!content.contains("expired"));
}

#[test]
fn unknown_op_ignored_for_forward_compat() {
    let text = concat!(
        r#"{"op":"add","ts":"2026-04-17T10:00:00Z","session_id":"a","cwd":"/tmp","claude_pid":1,"terminal_pid":null,"transcript_path":"/tmp/t","notification_type":"permission_prompt","message":"m"}"#,
        "\n",
        r#"{"op":"future_op","ts":"2026-04-17T10:01:00Z","session_id":"a","reason":"x"}"#,
        "\n",
    );
    let (ops, skipped) = parser::parse_lines(text);
    assert_eq!(ops.len(), 1);
    assert_eq!(skipped, 1);
}
```

- [ ] **Step 2: Ensure tests run**

Run: `cargo test --test end_to_end`
Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add tests/end_to_end.rs
git commit -m "test: add workspace-level end-to-end integration tests"
```

---

## Task 8: Inline Doc Comments on Core Public API

**Files:**
- Modify: `crates/core/src/lib.rs`, `crates/core/src/types.rs`, etc.

- [ ] **Step 1: Add crate-level doc to `crates/core/src/lib.rs`**

```rust
//! Core library for Claude Pending Board.
//!
//! This crate contains the platform-agnostic logic:
//! - [`board`] — JSONL parser, in-memory state store, file watcher, and compaction
//! - [`visibility`] — finite state machine controlling HUD visibility, cooldown,
//!   and reminding behavior
//! - [`reaper`] — periodic liveness check that promotes dead Claude Code processes
//!   to stale entries on the board
//! - [`terminal`] — adapter trait + process ancestor walk for terminal focus
//! - [`config`] — user-editable settings persisted to TOML
//! - [`types`] — shared domain types (`Entry`, `Op`, `NotificationType`, etc.)
//!
//! The core crate has no Tauri dependency — the Tauri app in `crates/app`
//! composes these pieces.

pub mod board;
pub mod config;
pub mod reaper;
pub mod terminal;
pub mod types;
pub mod visibility;
```

- [ ] **Step 2: Verify docs build without warnings**

Run: `cargo doc --no-deps -p claude-pending-board-core 2>&1 | grep -i warning`
Expected: no output (no warnings).

If there are warnings about missing docs, add `//!` or `///` comments to each flagged item.

- [ ] **Step 3: Commit**

```bash
git add crates/core/src/lib.rs
git commit -m "docs(core): add crate-level doc comments to public API"
```

---

## Task 9: Version Bump and v0.1.0 Tag

**Files:**
- Modify: workspace `Cargo.toml`
- Modify: `crates/app/tauri.conf.json`
- Modify: `plugin/.claude-plugin/plugin.json`

- [ ] **Step 1: Bump workspace version**

In `Cargo.toml` at the repo root, verify `[workspace.package] version = "0.1.0"`. If it's different, update it.

- [ ] **Step 2: Verify `crates/app/tauri.conf.json` version**

Open `crates/app/tauri.conf.json` — ensure `"version": "0.1.0"`.

- [ ] **Step 3: Verify `plugin/.claude-plugin/plugin.json` version**

Open `plugin/.claude-plugin/plugin.json` — ensure `"version": "0.1.0"`.

- [ ] **Step 4: Run the full release checklist**

Follow `docs/release-checklist.md` manually. Fix any issues that come up before tagging.

- [ ] **Step 5: Tag and push**

```bash
git tag -a v0.1.0 -m "Release v0.1.0 — first public release"
git push origin main
git push origin v0.1.0
```

- [ ] **Step 6: Verify release workflow**

Watch `https://github.com/sadwx/claude-pending-board/actions` — the release workflow should trigger and produce a draft release with artifacts for all three OSes.

- [ ] **Step 7: Publish the release**

Once the draft release is populated, edit it to add release notes, mark it as latest, and publish.

- [ ] **Step 8: Verify plugin installation works**

On a different machine (or a clean `~/.claude/`):
```bash
/plugin marketplace add github:sadwx/claude-pending-board
/plugin install claude-pending-board@claude-pending-board
/reload-plugins
/pending-board doctor
```

All doctor checks should pass (with tray app installed).

---

## Self-Review Checklist

1. **Spec coverage:**
   - Plugin installation via `/plugin install`: Task 1 plugin.json — covered
   - `/pending-board doctor` diagnostic: Task 2 — covered
   - Three hooks registered by plugin: Task 1 plugin.json hooks config — covered
   - CI runs fmt/clippy/test: Task 5 — covered
   - Release produces per-OS artifacts: Task 6 — covered
   - End-to-end integration tests: Task 7 — covered
   - Documentation: Tasks 3, 4, 8 — covered
   - v0.1.0 release: Task 9 — covered

2. **Placeholder scan:** No TBD/TODO. Release-checklist contains manual verification steps but all are concrete.

3. **Type consistency:** Plugin JSON schema uses `${CLAUDE_PLUGIN_ROOT}` consistently. Tauri action inputs match documented fields. End-to-end tests reference `board::parser::parse_lines`, `board::compaction::compact`, `board::store::StateStore::apply_all` — all match existing public API.
