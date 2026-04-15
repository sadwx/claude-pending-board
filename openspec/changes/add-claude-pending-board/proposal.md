# add-claude-pending-board

## Why

Claude Code sessions running in background terminal tabs frequently stall waiting for user input (permission prompts or idle after a turn). Today the user typically doesn't notice for minutes, alt-tabs through multiple WezTerm / iTerm2 tabs to find which session is waiting, and loses flow. The user wants a single cross-platform inbox that surfaces every pending session across all projects, pulls attention when new items arrive, and takes them directly to the owning terminal pane with one click.

## What Changes

- **NEW** Cross-platform Rust tray application (`claude-pending-board`) built on Tauri 2 with a fixed 380×440 draggable floating HUD window.
- **NEW** Hook scripts (PowerShell on Windows, Bash on macOS / Linux) that subscribe to Claude Code `Notification`, `UserPromptSubmit`, and `Stop` events and append ops to an append-only JSONL board file at `~/.claude/pending/board.jsonl`.
- **NEW** File-driven state model: the HUD watches the JSONL file and replays operations to reconstruct an in-memory list of pending sessions. Periodic reaper verifies liveness via OS process checks.
- **NEW** Auto show/hide visibility state machine: HUD auto-shows on 0→1 transition, auto-hides after a 2-second grace period on last clear, supports manual-dismiss with configurable cooldown (default 15 minutes) and a reminding toggle.
- **NEW** Per-dismiss confirmation panel with a 5-second countdown, default-action override, and split-line helper captions.
- **NEW** Terminal adapter abstraction with two v1 implementations: a WezTerm adapter (Windows / macOS / Linux, via `wezterm cli`) and an iTerm2 adapter (macOS only, via `osascript`). Click action focuses the owning pane on live entries and spawns `claude --resume <session_id>` in a new tab on stale entries.
- **NEW** Live / stale entry distinction with per-state icons, sorting rules (permission > idle > stale, then newest-first within each group), and section dividers.
- **NEW** Settings window for adjusting cooldown duration, reminding toggle, grace delay, confirmation countdown, default terminal adapter, and HUD position reset.
- **NEW** Claude Code plugin delivery channel exposing a `/pending-board` slash command (`status`, `install`, `doctor`, `hooks-uninstall`) alongside a Settings-driven manual installer inside the Tauri app.
- **NEW** README and INSTALL documentation aimed at end users.

## Capabilities

### New Capabilities

- `pending-board`: The end-to-end behavior users experience — hook-driven capture of pending Claude Code sessions, scrollable sorted list in a floating HUD, auto show/hide visibility with cooldown, dismiss confirmation with split-line emphasis, click-to-focus via terminal adapters, and stale resume via `claude --resume`.

### Modified Capabilities

_None — this is a greenfield repository._

## Impact

- **Affected code**: new repository `claude-pending-board` (Cargo workspace). No existing code changes.
- **External systems**: reads `~/.claude/pending/board.jsonl`, `~/.claude/sessions/<pid>.json`, `~/.claude/pending/config.toml`; writes logs to `~/.claude/pending/logs/`. Does not touch `~/.claude/settings.json` unless the user explicitly runs the install action (which first backs up the file).
- **Dependencies**: users must have Claude Code installed (any version with the `Notification`, `UserPromptSubmit`, and `Stop` hook events) and WezTerm (Windows / Linux) or iTerm2 (macOS) in `PATH`.
- **Distribution**: GitHub Releases for the Tauri binary; a Claude Code plugin marketplace entry for the hook installer and slash command.
