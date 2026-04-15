# Design

## Context

Claude Code (`claude`) is a CLI that commonly runs in several terminal tabs in parallel across multiple project working directories. Each session pauses periodically to ask for permission or to wait for the user's next instruction. Today there is no OS-level surface that aggregates "which Claude Code sessions are currently waiting for me?" across all tabs, and missed prompts compound into minutes of idle time.

The starting point is rich enough to build on without inventing anything new:

- Claude Code's hooks framework fires `Notification` events with `session_id`, `cwd`, `transcript_path`, `hook_event_name`, `message`, and a `notification_type` of either `permission_prompt` or `idle_prompt`. A `UserPromptSubmit` event fires when the user answers and a `Stop` event fires when a turn ends.
- Claude Code writes `~/.claude/sessions/<PID>.json` for every live CLI session, containing `pid`, `sessionId`, `cwd`, and `entrypoint`. This makes reliable liveness detection possible with zero extra plumbing.
- WezTerm ships a first-class `wezterm cli list` / `activate-pane` / `spawn` protocol that cross-platform Rust code can shell out to. iTerm2 exposes an AppleScript surface rich enough to do the equivalent.
- The user's target terminal set is WezTerm (Windows / Linux) and iTerm2 (macOS); Windows Terminal is explicitly out of scope because it cannot programmatically focus a specific tab.
- The user is Simon Lin, working primarily in a PowerShell 7 environment on Windows 11 with Claude Code remote-control sessions enabled. Rust is a hard requirement for every piece of the stack the user writes.

## Goals / Non-Goals

**Goals**

- Zero silent-idleness: every `permission_prompt` and `idle_prompt` must reach a visible surface within a few seconds of firing.
- Single unified inbox across every project directory.
- One-click jump from an entry to the exact WezTerm / iTerm2 pane that owns the session, including a sensible fallback to resume (`claude --resume <session_id>`) in a new tab when the original pane is gone.
- Cross-platform single-codebase in Rust: Windows, macOS, Linux all from the same `Cargo` workspace.
- Non-activating HUD: the app never steals keyboard focus.
- Dual delivery: downloadable Tauri app binary + Claude Code plugin.
- Configurable timings (cooldown, grace, countdown) and terminal adapter.

**Non-Goals**

- Windows Terminal adapter (no API to activate a specific tab).
- Other terminals (Alacritty, Kitty, ghostty, VS Code integrated terminal, tmux-in-terminal).
- Light theme (v1 is dark-only, Catppuccin Mocha).
- i18n / RTL layout.
- Screen-reader accessibility and full keyboard navigation inside the HUD (Esc-to-dismiss is the only keyboard affordance).
- Deep-linking to `claude.ai/code/session_<ulid>` (the remote ULID is not persisted on disk by the CLI).
- Cross-machine or networked board sharing (the board file is strictly local).
- Encryption of the board file.
- Toast notifications, sound by default, or any other surface besides the floating HUD.

## Decisions

### UI framework — Tauri 2

Chosen over `egui` and `Iced`. The app needs a real native tray icon, multiple windows (HUD + Settings), OS-native notification APIs (even though we disable toast, the `FlashWindowEx` / `requestUserAttention` demand-attention path is simplest via Tauri), a single-instance lock, and an autostart plugin. Tauri 2 offers first-class plugins for all of these. `egui` / `Iced` would need hand-rolled integration for tray and windowing on each OS, and the UI itself would be custom-drawn rather than native-feeling. The "Rust for the entire development" requirement is satisfied because Tauri's UI layer is a thin HTML/CSS/TS-lite shell (no React, no Svelte — vanilla DOM with Tauri `invoke()` bridging to the Rust core). All business logic stays in Rust crates.

### Data source — append-only JSONL board file at `~/.claude/pending/board.jsonl`

Chosen over SQLite and named pipes / sockets. JSONL is dead simple from PowerShell and Bash (a single `Add-Content` / `printf >>` call), crash-safe because `O_APPEND` writes below `PIPE_BUF` (~4 KB) are atomic, human-readable for debugging, and survives the app being offline. SQLite would add a dependency to both hook script families (adding `sqlite3` to PowerShell workflows is painful). Named pipes / sockets require the app to be running at hook-fire time, violating the "hook must never block Claude" rule.

The file stores three op types: `add`, `clear`, `stale`. State is reconstructed by replaying ops in order. Compaction runs on startup and when the file exceeds 5 MB or 10 000 lines. Compaction is atomic via write-to-`.tmp` + rename.

### Process topology — single Tauri process

Chosen over a daemon + UI split and a file-free direct IPC. The Tauri app owns the `BoardWatcher`, `StateStore`, `VisibilityController`, `Reaper`, adapter registry, and both windows. A daemon split buys nothing at this scale because Tauri windows can hide without killing the process. A direct-IPC design would lose events when the app is not running.

### Hook script language — platform-native shells

PowerShell on Windows, Bash on macOS / Linux. Rejected a compiled Rust hook binary because cold-start cost on Windows (~30 ms per hook invocation vs. ~5 ms for a shell script) matters when hooks fire multiple times per turn. Scripts are also easier for the user to inspect and modify. The scripts live as the source of truth under `scripts/` and are copied into the Claude Code plugin directory on install.

### Hook-failure policy — silently fail, log to file

A hook that errors must never block Claude Code. Every script wraps its entire body in a top-level `try` / `catch`, logs failures to `~/.claude/pending/logs/hook-errors.log`, and exits 0. The alternative (surfacing failures to the user via OS notification) was considered and rejected — it adds complexity for a failure mode that is already self-recovering (the next hook fire writes a fresh line).

### Visibility behavior — finite state machine owned by `VisibilityController`

The HUD appears only on a 0 → 1 transition. Further `add` events while shown are silent except for list re-renders and tray badge updates. The HUD auto-hides after a 2-second grace period once the board goes empty (grace prevents flicker during chained permission prompts). Manual dismiss transitions to `CooldownHidden` for the configured cooldown (default 15 minutes); during the cooldown window, new `add` events set a `seen_add` flag but do not re-show the HUD. When the cooldown expires, the HUD re-shows if and only if the global Reminding toggle is on AND at least one add event fired during the cooldown. A per-dismiss override lets the user flip the reminding decision for just this one dismiss via the confirmation panel.

### Dismiss confirmation — inline transient panel, not modal

When the user clicks dismiss, the HUD content swaps for a 5-second confirmation panel. The default button (matching the global Reminding setting) is visually highlighted with a pink accent border, a `DEFAULT` pill, and a live `Wake me · 5s` countdown inside the button. Helper captions below each button describe consequences (`Choose this to wake me / after 15 minutes` and `Choose this to stay silent`). Esc or waiting out the countdown applies the default. A single click on either button commits immediately. This is preferred over a modal dialog because the HUD itself is non-activating and should not produce a modal that grabs focus.

### Click-to-focus — terminal adapter trait + process ancestor walking

A `TerminalAdapter` trait lives in `crates/core` with methods `detect(pid)`, `focus_pane(match)`, and `spawn_resume(cwd, session_id)`. Two implementations ship in v1: `WezTermAdapter` (shells out to `wezterm cli list --format json`, matches pane `pid` against the ancestor walk from `claude_pid`, calls `activate-pane` + brings window forward) and `ITerm2Adapter` (Mac-only, uses `osascript` to activate the app and focus the session whose `tty` matches the ancestor's `tty`). Detection walks from `claude_pid` up through the process tree using `sysinfo`, stopping at the first ancestor whose process name matches a known terminal binary.

For stale entries, click falls through to `spawn_resume` on the user's default adapter: `wezterm cli spawn --cwd <cwd> -- claude --resume <session_id>` or the iTerm2 AppleScript equivalent. A 10-second "resolving" window suppresses the stale entry after click to avoid a double-flash when the new session's first hook fires.

### Sorting — compound (type priority, then ts desc)

Primary: `permission_prompt` (red) > `idle_prompt` (blue) > `stale` (grey). Secondary: `ts` descending within each group. Type-group dividers separate sections visually.

### Reaper — periodic liveness sweep

A `tokio` task runs every 30 seconds. For each live entry it checks (1) the `claude_pid` is still in the OS process list, and (2) `~/.claude/sessions/<claude_pid>.json` still exists with a matching `sessionId`. Both checks are required to guard against PID recycling. If either fails, the entry is promoted to `Stale` (not deleted). Entries that have been `Stale` for more than 24 hours are dropped during the next compaction.

### Repository layout — Cargo workspace, single repo

```
claude-pending-board/
├── Cargo.toml                  # workspace root
├── crates/
│   ├── core/                   # platform-agnostic logic, no Tauri
│   ├── adapters/               # WezTerm + iTerm2 + ancestor walking
│   └── app/                    # Tauri 2 binary
├── scripts/                    # source of truth for hook scripts
├── plugin/                     # Claude Code plugin (hooks/, commands/)
├── openspec/                   # specs and change proposals
├── docs/
├── README.md
└── INSTALL.md
```

Three crates: `core` (pure Rust, no Tauri), `adapters` (terminal-specific), `app` (Tauri shell that composes the others). The HUD UI lives as vanilla HTML/CSS/TS-lite inside `crates/app/ui/`.

### Telemetry & logs

Everything goes to `~/.claude/pending/logs/` as daily-rotated files via `tracing-appender`, with 14-day retention. The Settings window exposes a debug-verbosity toggle.

## Risks / Trade-offs

- **[PID recycling leads to false-live entries]** → Require both pid-alive AND matching `sessions/<pid>.json` content before keeping an entry live. Reaper treats a mismatch as stale.
- **[Hook fires but the app is not running]** → Accepted. The board file accumulates ops; startup compaction reads the whole file and rebuilds state cleanly.
- **[Process ancestor walk returns no known terminal (e.g. VS Code integrated terminal)]** → Entry goes on the board with `terminal_pid: null`. Click action falls through to `spawn_resume` using the default adapter; the user can still reach the session.
- **[WezTerm / iTerm2 binary not installed]** → Settings shows a warning banner on launch; click action shows an inline HUD banner with install instructions. No crash path.
- **[Board file write races between parallel hook invocations]** → Each line is a self-contained record; POSIX `O_APPEND` and NTFS append writes below ~4 KB are atomic. Watcher reads only up to EOF-at-notify-time and keeps an offset cursor. Malformed half-lines are skipped and dropped at next compaction.
- **[Tauri webview crashes bring down the whole process]** → Accepted for v1. Single-instance lock releases cleanly on crash; user relaunches from Start Menu / Spotlight.
- **[Focus-steal behavior differs across OSes]** → Addressed by the non-activating window flags per platform (`WS_EX_NOACTIVATE`, `canBecomeKey = false`, `_NET_WM_STATE_ABOVE` + skip-taskbar). Tested manually via the release checklist.
- **[Users forget to install hook scripts after downloading the app]** → Settings window's "Install hooks" button is surfaced as a first-run banner. Doctor slash command in the plugin diagnoses missing registrations.
- **[Remote `session_id` (ULID) is not reachable from hooks]** → Out of scope. Documented in non-goals.
- **[OpenSpec adoption adds a dependency on a new CLI]** → Accepted. OpenSpec artifacts are plain Markdown; the CLI is used for scaffolding and validation, not for runtime behavior.
