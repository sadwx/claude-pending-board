# pending-board

This is the **working spec** at v0.1.2 — a snapshot of behaviors that are merged and shipped. The source change folder is archived at `openspec/changes/archive/add-claude-pending-board/`. Subsequent change proposals (e.g. `openspec/changes/add-wsl-support/`) describe deltas on top of this document.

## Requirements

### Requirement: Hook-driven entry capture

The system SHALL capture every `permission_prompt` and `idle_prompt` notification fired by Claude Code as a new entry on the pending board, keyed by `session_id`.

#### Scenario: Permission prompt becomes a pending entry

- **WHEN** Claude Code fires a `Notification` hook event with `notification_type = "permission_prompt"` and a non-empty `session_id`
- **THEN** the installed hook script SHALL append a JSON line of shape `{"op":"add","ts":<iso>,"session_id":<id>,"cwd":<path>,"claude_pid":<int>,"terminal_pid":<int|null>,"transcript_path":<path>,"notification_type":"permission_prompt","message":<string>}` to `~/.claude/pending/board.jsonl`
- **AND** the Tauri app's `BoardWatcher` SHALL observe the file change and insert the entry into the in-memory `StateStore`

#### Scenario: Idle prompt becomes a pending entry

- **WHEN** Claude Code fires a `Notification` hook event with `notification_type = "idle_prompt"`
- **THEN** the hook SHALL write an equivalent `add` op with `notification_type = "idle_prompt"` to `board.jsonl`

#### Scenario: Hook write failure does not block Claude Code

- **WHEN** the hook script encounters any error while preparing or writing the board line (missing directory, disk full, permission denied, malformed stdin JSON, internal script bug)
- **THEN** the script SHALL log the failure to `~/.claude/pending/logs/hook-errors.log` and exit with status 0
- **AND** Claude Code SHALL NOT be blocked or interrupted in any way

### Requirement: Entry removal

The system SHALL remove a pending entry from the board when the user answers, when the turn ends, or when the session itself terminates — via the `UserPromptSubmit`, `Stop`, and `SessionEnd` hooks respectively.

#### Scenario: User answers the prompt

- **WHEN** Claude Code fires a `UserPromptSubmit` hook event for a session with `session_id`
- **THEN** the hook SHALL append `{"op":"clear","ts":<iso>,"session_id":<id>,"reason":"user_replied"}` to `board.jsonl`
- **AND** the `StateStore` SHALL remove the entry for that `session_id`

#### Scenario: Claude Code finishes its turn

- **WHEN** Claude Code fires a `Stop` hook event
- **THEN** the hook SHALL append `{"op":"clear","ts":<iso>,"session_id":<id>,"reason":"stop"}` to `board.jsonl`
- **AND** the `StateStore` SHALL remove the entry for that `session_id`

#### Scenario: Session ends (`/clear`, `/compact`, normal exit)

- **WHEN** Claude Code fires a `SessionEnd` hook event for a session with `session_id` (the *terminating* session's id, not the new one for `/clear` and `/compact`)
- **THEN** the hook SHALL append `{"op":"clear","ts":<iso>,"session_id":<id>,"reason":"session_ended"}` to `board.jsonl`
- **AND** the `StateStore` SHALL remove the entry for that `session_id`

This covers `/clear` specifically — it does NOT fire `Stop`, only `SessionEnd` for the old session followed by `SessionStart` for the new one. `/compact`, normal exit, and other termination paths are also covered uniformly.

#### Scenario: Clear op for unknown session_id is a no-op

- **WHEN** the `BoardWatcher` observes a `clear` op for a `session_id` that is not currently in the `StateStore`
- **THEN** the `StateStore` SHALL log the event at debug level and make no state changes

### Requirement: Live / stale liveness tracking

The system SHALL continuously verify that every live entry on the board corresponds to a still-running Claude Code process, and promote dead entries to the `stale` state.

#### Scenario: Claude Code process still alive

- **WHEN** the Reaper runs its periodic check (every 30 seconds) on a live entry with `claude_pid = P`
- **AND** process `P` exists in the OS process table
- **AND** `~/.claude/sessions/P.json` exists with a `sessionId` matching the entry's `session_id`
- **THEN** the entry SHALL remain in the `live` state

#### Scenario: Process died — entry promoted to stale

- **WHEN** the Reaper runs on a live entry whose `claude_pid` is no longer in the process table
- **THEN** the Reaper SHALL append `{"op":"stale","ts":<iso>,"session_id":<id>,"reason":"pid_dead"}` to `board.jsonl`
- **AND** mutate the entry's state to `stale` in the `StateStore`

#### Scenario: PID recycled to an unrelated process

- **WHEN** the Reaper runs on a live entry
- **AND** `claude_pid = P` is alive but `~/.claude/sessions/P.json` does not exist or its `sessionId` does not match the entry
- **THEN** the Reaper SHALL write a `stale` op with `reason = "session_file_missing"` or `"mismatch"` respectively and mutate the entry

### Requirement: Stale entry expiration and cleanup

The system SHALL eventually remove orphaned stale entries — entries whose owning session was abandoned and replaced by a different one — without requiring user action. The fixed expiry is **1 hour** from the moment an entry first transitioned to `stale`.

#### Scenario: Periodic cleanup emits clear ops for expired stale entries

- **WHEN** the periodic stale-cleanup loop runs (every 10 minutes by default)
- **AND** there exists at least one entry with `state = "stale"` and `now - stale_since > 1 hour`
- **THEN** the loop SHALL append `{"op":"clear","ts":<iso>,"session_id":<id>,"reason":"stale_expired"}` for each such entry to `board.jsonl`
- **AND** the `BoardWatcher` SHALL pick those up through the standard pipeline and the entries SHALL be removed from the `StateStore`

#### Scenario: Startup compaction drops expired stale entries

- **WHEN** the tray app starts and `board.jsonl` exists
- **THEN** the compaction routine SHALL replay all ops, drop any entry whose `state = "stale"` and `now - stale_since > 1 hour`, and rewrite `board.jsonl` atomically with only the surviving entries

#### Scenario: Recently-stale entries are preserved

- **WHEN** an entry has been in the `stale` state for less than 1 hour
- **THEN** neither the periodic cleanup nor startup compaction SHALL remove it
- **AND** the entry SHALL remain visible in the HUD with stale styling, available for click-to-resume or per-entry dismiss

### Requirement: Sorting and grouping

The system SHALL display entries sorted by type priority (permission > idle > stale) with newest-first ordering within each type group.

#### Scenario: Mixed entry types are grouped and ordered

- **WHEN** the HUD renders a list containing 2 permission_prompt entries, 2 idle_prompt entries, and 1 stale entry
- **THEN** the order SHALL be: newest permission, older permission, newest idle, older idle, stale

### Requirement: Floating HUD window

The system SHALL present pending entries in a fixed-size floating window that does not steal keyboard focus from whatever application the user was working in.

#### Scenario: HUD dimensions and chrome

- **WHEN** the HUD is visible
- **THEN** it SHALL be 380 × 440 pixels, non-resizable, draggable by its header bar, with rounded corners and a drop shadow
- **AND** the header SHALL contain a logo glyph, the title "Pending Board", a count badge, a settings gear, and a dismiss `×` button

#### Scenario: List scrolls when overflowing

- **WHEN** the number of entries exceeds what fits in the visible list area
- **THEN** the list SHALL scroll vertically inside the HUD without changing the window size

#### Scenario: HUD show preserves caller focus

- **WHEN** the HUD becomes visible — whether from auto-show, manual tray click, or any other path
- **THEN** the application that previously held keyboard focus SHALL retain it
- **AND** any keystrokes the user types in the moment of HUD appearance SHALL go to that previous application, not to the HUD

### Requirement: Auto show / hide behavior

The system SHALL automatically show the HUD when the first pending entry arrives and hide it when the board becomes empty.

#### Scenario: Auto-show on first entry

- **WHEN** the `StateStore` transitions from 0 entries to 1 or more entries
- **AND** the visibility state is `Hidden` (not `CooldownHidden`)
- **THEN** the `VisibilityController` SHALL show the HUD

#### Scenario: Auto-hide after grace period

- **WHEN** the `StateStore` transitions from 1 or more entries to 0 entries
- **THEN** the `VisibilityController` SHALL start a 2-second grace timer
- **AND** if no `add` op arrives before the timer expires, the HUD SHALL be hidden
- **AND** if an `add` op arrives during the grace, the timer SHALL be cancelled and the HUD SHALL remain visible

#### Scenario: Additional adds while shown do not re-animate

- **WHEN** the HUD is already `Shown` and a new `add` op arrives (count goes from 3 to 4)
- **THEN** the list SHALL re-render with the new entry but the window SHALL NOT re-show, flash, or otherwise re-animate beyond a brief row-highlight on the new row

### Requirement: Manual dismiss with cooldown and reminding

The system SHALL support manual dismissal of the HUD with a configurable cooldown during which auto-show is suppressed, and an optional reminder at cooldown expiry.

#### Scenario: Manual dismiss enters cooldown

- **WHEN** the user clicks the dismiss `×` and the confirmation panel commits
- **THEN** the `VisibilityController` SHALL transition to `CooldownHidden` with a timer equal to the configured cooldown (default 15 minutes) and a `seen_add` flag initialized to `false`

#### Scenario: New adds during cooldown set the seen_add flag

- **WHEN** the visibility state is `CooldownHidden` and a new `add` op arrives
- **THEN** the HUD SHALL NOT be shown
- **AND** the `seen_add` flag SHALL be set to `true`
- **AND** the tray badge count SHALL update

#### Scenario: Reminder fires at cooldown expiry when enabled and items accumulated

- **WHEN** the cooldown timer expires
- **AND** the Reminding toggle is enabled
- **AND** `seen_add` is `true`
- **THEN** the HUD SHALL auto-show

#### Scenario: No reminder when Reminding is disabled

- **WHEN** the cooldown timer expires
- **AND** the Reminding toggle is disabled
- **THEN** the HUD SHALL remain hidden regardless of `seen_add`

#### Scenario: No reminder when nothing changed

- **WHEN** the cooldown timer expires
- **AND** `seen_add` is `false`
- **THEN** the HUD SHALL remain hidden

#### Scenario: Manual open cancels cooldown

- **WHEN** the visibility state is `CooldownHidden` and the user clicks the tray icon
- **THEN** the cooldown SHALL be cancelled and the HUD SHALL be shown

### Requirement: Dismiss confirmation panel

The system SHALL present a 5-second confirmation panel on manual dismiss with a clearly-highlighted default action and per-dismiss override of the global Reminding setting.

#### Scenario: Confirmation appears on dismiss click

- **WHEN** the user clicks the HUD dismiss `×` and the `skip_dismiss_confirmation` setting is `false`
- **THEN** the HUD list area SHALL be replaced with a confirmation panel while the header remains visible
- **AND** the panel SHALL show a heading describing the upcoming hide duration
- **AND** the panel SHALL show two buttons: a "Wake me" option and a "Stay silent" option, with the one matching the current Reminding default visually highlighted and showing a countdown badge

#### Scenario: Skip-confirm bypasses the panel entirely

- **WHEN** the user clicks the HUD dismiss `×` and the `skip_dismiss_confirmation` setting is `true`
- **THEN** the HUD SHALL apply the current global Reminding setting immediately (equivalent to a 0-second countdown firing the default action)
- **AND** SHALL NOT flash the confirmation panel at any point

#### Scenario: Countdown expires with default

- **WHEN** the 5-second countdown reaches 0 with no user interaction
- **THEN** the action corresponding to the current global Reminding setting SHALL be applied and the HUD SHALL transition to `CooldownHidden`

#### Scenario: Esc keystroke applies default

- **WHEN** the confirmation panel is visible and the user presses Esc
- **THEN** the same behavior as the countdown expiring SHALL apply

#### Scenario: User clicks an override button

- **WHEN** the confirmation panel is visible and the user clicks either button
- **THEN** the countdown SHALL be cancelled immediately
- **AND** the clicked action SHALL be applied (overriding the global Reminding setting for this dismiss only)
- **AND** the visibility state SHALL transition to `CooldownHidden` with `reminding_override` set accordingly

### Requirement: Per-entry dismiss

The system SHALL allow the user to dismiss a single entry from the HUD without affecting any other entry, for cases where one row is no longer relevant (commonly an orphaned stale entry).

#### Scenario: Dismiss button appears on hover

- **WHEN** the user hovers an entry row in the HUD
- **THEN** a small `×` button SHALL fade in on the right side of that row
- **AND** the button SHALL be otherwise hidden so it does not clutter the resting list

#### Scenario: Click on the dismiss button removes the entry

- **WHEN** the user clicks the per-entry `×` button
- **THEN** an op of shape `{"op":"clear","ts":<iso>,"session_id":<id>,"reason":"user_dismissed"}` SHALL be appended to `board.jsonl`
- **AND** the entry SHALL be removed from the `StateStore` through the standard pipeline
- **AND** the row click that ordinarily triggers focus / resume SHALL NOT also fire

### Requirement: Click to focus live terminal pane

The system SHALL focus the exact terminal pane owning a live entry when the user clicks that entry.

#### Scenario: WezTerm pane is focused

- **WHEN** the user clicks a live entry whose ancestor walk from `claude_pid` matches a `wezterm-gui` process
- **THEN** the `WezTermAdapter` SHALL call `wezterm cli list --format json`, find the pane whose `pid` matches an ancestor in the walk, and call `wezterm cli activate-pane --pane-id <matched>`
- **AND** the WezTerm top-level window SHALL be brought to the foreground

#### Scenario: iTerm2 session is focused

- **WHEN** the user clicks a live entry on macOS whose ancestor walk matches an `iTerm2` process
- **THEN** the `ITerm2Adapter` SHALL activate iTerm2 via `osascript` and select the session whose `tty` matches the ancestor walk's terminal tty

#### Scenario: No adapter matched

- **WHEN** the ancestor walk returns no known terminal binary
- **THEN** the click SHALL fall through to the user's default adapter via `spawn_resume` rather than failing silently

### Requirement: Click to resume stale entry

The system SHALL resume a stale session in a new terminal tab by invoking `claude --resume <session_id>` via the user's default adapter.

#### Scenario: Stale WezTerm entry resumed

- **WHEN** the user clicks a stale entry and the default adapter is WezTerm
- **THEN** the adapter SHALL run `wezterm cli spawn --cwd <original_cwd> -- claude --resume <session_id>`

#### Scenario: Stale iTerm2 entry resumed

- **WHEN** the user clicks a stale entry on macOS and the default adapter is iTerm2
- **THEN** the adapter SHALL invoke `osascript` to run `tell application "iTerm2" to tell current window to create tab with default profile command "cd <cwd> && claude --resume <session_id>"`

### Requirement: Settings surface

The system SHALL expose a Settings window for user-editable behavior, persisted to `~/.claude/pending/config.toml`.

#### Scenario: Settings window opens from the HUD

- **WHEN** the user clicks the gear icon in the HUD header OR selects "Settings…" from the tray context menu
- **THEN** a separate, resizable Settings window SHALL open (not auto-shown, not part of the visibility state machine)

#### Scenario: Settings fields

- **WHEN** the Settings window is visible
- **THEN** it SHALL show editable fields for: cooldown after manual dismiss (1–120 min slider, default 15), Reminding enabled (toggle, default on), auto-hide grace delay (0–10 s slider, default 2), dismiss confirmation countdown (2–10 s slider, default 5), skip dismiss confirmation (toggle, default off), default terminal adapter (per-platform dropdown), and a "Reset HUD position" button

#### Scenario: Live config reload

- **WHEN** the user changes any setting and saves
- **THEN** `config.toml` SHALL be written atomically
- **AND** the new values SHALL be applied immediately without requiring an app restart

### Requirement: Hook installation paths

The system SHALL provide two equivalent installation paths for the hook scripts: a Claude Code plugin and a first-run setup card inside the HUD that drives the same plugin install via `claude plugin` CLI.

#### Scenario: Plugin marketplace install registers hooks

- **WHEN** the user runs `claude plugin marketplace add sadwx/claude-pending-board` followed by `claude plugin install claude-pending-board@claude-pending-board` (or the equivalent slash commands inside Claude Code)
- **THEN** the marketplace catalog at `.claude-plugin/marketplace.json` SHALL list `claude-pending-board` with `source = "./plugin"`
- **AND** the plugin's `plugin.json` SHALL register the four hooks (`Notification`, `UserPromptSubmit`, `Stop`, `SessionEnd`) pointing to platform-appropriate scripts bundled inside the plugin
- **AND** no changes SHALL be made to the user's global `~/.claude/settings.json`

#### Scenario: First-run setup card in HUD

- **WHEN** the tray app launches and the plugin is not yet installed (verified via `claude plugin list`)
- **AND** the user opens the HUD
- **THEN** the HUD SHALL display a setup card in place of the empty state, with title "Hooks not installed", an explanatory subtitle, an `[Install plugin]` primary button, and a `[Do it manually]` secondary button

#### Scenario: One-click install from the setup card

- **WHEN** the user clicks `[Install plugin]` on the setup card
- **THEN** the app SHALL shell out to `claude plugin marketplace add sadwx/claude-pending-board` and then `claude plugin install claude-pending-board@claude-pending-board` running as the user
- **AND** on success the setup card SHALL self-clear and the HUD SHALL render the regular empty state
- **AND** on failure (e.g. `claude` CLI not in PATH) an inline error SHALL be shown with the stderr from the CLI and the manual instructions SHALL remain accessible

#### Scenario: Doctor diagnoses a broken install

- **WHEN** the user runs `/pending-board doctor`
- **THEN** the plugin SHALL verify that the three hooks are registered, that the script files exist and are executable, that `~/.claude/pending/board.jsonl` is writable, and that the configured terminal adapter binary is in `PATH`
- **AND** SHALL report any failed checks with remediation hints

### Requirement: Board file resilience

The system SHALL tolerate malformed lines, truncated writes, schema additions, and file deletion without losing state or crashing.

#### Scenario: Malformed JSON line is skipped

- **WHEN** the `BoardWatcher` reads a line that fails to parse as JSON
- **THEN** it SHALL log a warning with the line offset and continue processing subsequent lines

#### Scenario: Unknown op is ignored

- **WHEN** a line parses successfully but its `op` field is not one of `add`, `clear`, `stale`
- **THEN** it SHALL be ignored silently for forward compatibility

#### Scenario: File deleted during runtime

- **WHEN** `board.jsonl` is deleted while the app is running
- **THEN** the app SHALL log a warning, clear in-memory state, and continue observing for re-creation

#### Scenario: Compaction is atomic

- **WHEN** the compaction routine runs (file > 5 MB or > 10 000 lines, or at startup)
- **THEN** the new content SHALL be written to `board.jsonl.tmp` and renamed over `board.jsonl` in a single step
