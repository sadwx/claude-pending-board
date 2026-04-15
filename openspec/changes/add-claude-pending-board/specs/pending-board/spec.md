# pending-board

## ADDED Requirements

### Requirement: Hook-driven entry capture

The system SHALL capture every `permission_prompt` and `idle_prompt` notification fired by Claude Code as a new entry on the pending board, keyed by `session_id`.

#### Scenario: Permission prompt becomes a pending entry
- **WHEN** Claude Code fires a `Notification` hook event with `notification_type = "permission_prompt"` and a non-empty `session_id`
- **THEN** the installed hook script SHALL append a JSON line of shape `{"op":"add","ts":<iso>,"session_id":<id>,"cwd":<path>,"claude_pid":<int>,"terminal_pid":<int|null>,"transcript_path":<path>,"notification_type":"permission_prompt","message":<string>}` to `~/.claude/pending/board.jsonl`
- **AND** the Tauri app's `BoardWatcher` SHALL observe the file change within 100 ms and insert the entry into the in-memory `StateStore`

#### Scenario: Idle prompt becomes a pending entry
- **WHEN** Claude Code fires a `Notification` hook event with `notification_type = "idle_prompt"`
- **THEN** the hook SHALL write an equivalent `add` op with `notification_type = "idle_prompt"` to `board.jsonl`

#### Scenario: Hook write failure does not block Claude Code
- **WHEN** the hook script encounters any error while preparing or writing the board line (missing directory, disk full, permission denied, malformed stdin JSON, internal script bug)
- **THEN** the script SHALL log the failure to `~/.claude/pending/logs/hook-errors.log` and exit with status 0
- **AND** Claude Code SHALL NOT be blocked or interrupted in any way

### Requirement: Entry removal

The system SHALL remove a pending entry from the board when the user answers or when the turn ends, via the `UserPromptSubmit` and `Stop` hooks.

#### Scenario: User answers the prompt
- **WHEN** Claude Code fires a `UserPromptSubmit` hook event for a session with `session_id`
- **THEN** the hook SHALL append `{"op":"clear","ts":<iso>,"session_id":<id>,"reason":"user_replied"}` to `board.jsonl`
- **AND** the `StateStore` SHALL remove the entry for that `session_id`

#### Scenario: Claude Code finishes its turn
- **WHEN** Claude Code fires a `Stop` hook event
- **THEN** the hook SHALL append `{"op":"clear","ts":<iso>,"session_id":<id>,"reason":"stop"}` to `board.jsonl`
- **AND** the `StateStore` SHALL remove the entry for that `session_id`

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

#### Scenario: Stale entry expires after 24 hours
- **WHEN** an entry has been in the `stale` state for more than 24 hours
- **THEN** the next board compaction SHALL remove it

### Requirement: Sorting and grouping

The system SHALL display entries sorted by type priority (permission > idle > stale) with newest-first ordering within each type group.

#### Scenario: Mixed entry types are grouped and ordered
- **WHEN** the HUD renders a list containing 2 permission_prompt entries, 2 idle_prompt entries, and 1 stale entry
- **THEN** the order SHALL be: newest permission, older permission, newest idle, older idle, stale
- **AND** the list SHALL include a small uppercase section label and thin divider before each non-empty group

### Requirement: Floating HUD window

The system SHALL present pending entries in a fixed-size floating window that does not steal keyboard focus.

#### Scenario: HUD dimensions and chrome
- **WHEN** the HUD is visible
- **THEN** it SHALL be 380 × 440 pixels, non-resizable, draggable by a 44-pixel header bar, with a 10-pixel corner radius and drop shadow
- **AND** the header SHALL contain a status dot, the title "Claude Pending", a count badge, a settings gear, and a dismiss `×` button

#### Scenario: List scrolls when overflowing
- **WHEN** the number of entries exceeds what fits in the visible list area
- **THEN** the list SHALL scroll vertically inside the HUD without changing the window size

#### Scenario: Non-activating window
- **WHEN** the HUD appears, whether manually or automatically
- **THEN** it SHALL NOT steal keyboard focus from the currently-active application

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
- **THEN** the list SHALL re-render with the new entry but the window SHALL NOT re-show, flash, or otherwise re-animate beyond a brief 150 ms row-highlight on the new row

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
- **WHEN** the user clicks the HUD dismiss `×`
- **THEN** the HUD list area SHALL be replaced with a confirmation panel while the header remains visible
- **AND** the panel SHALL show the heading "Going silent for <cooldown> minutes"
- **AND** the panel SHALL show a dim subtitle "<N> items stay on board" where N is the current entry count
- **AND** the panel SHALL show two large buttons: one matching the Reminding default visually highlighted with a pink accent border, a `DEFAULT` pill, and an inline `<label> · Ns` countdown; the other flat

#### Scenario: Helper captions below buttons
- **WHEN** the confirmation panel is visible
- **THEN** a helper caption below the default button SHALL read `Choose this to wake me` on the first line and `after <cooldown> minutes` on the second line
- **AND** a helper caption below the non-default button SHALL read `Choose this to stay silent`

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
- **THEN** the HUD SHALL show an inline banner `No known terminal found` and offer to fall through to the user's default adapter via `spawn_resume`

### Requirement: Click to resume stale entry

The system SHALL resume a stale session in a new terminal tab by invoking `claude --resume <session_id>` via the user's default adapter.

#### Scenario: Stale WezTerm entry resumed
- **WHEN** the user clicks a stale entry and the default adapter is WezTerm
- **THEN** the adapter SHALL run `wezterm cli spawn --cwd <original_cwd> -- claude --resume <session_id>`

#### Scenario: Stale iTerm2 entry resumed
- **WHEN** the user clicks a stale entry on macOS and the default adapter is iTerm2
- **THEN** the adapter SHALL invoke `osascript` to run `tell application "iTerm2" to tell current window to create tab with default profile command "cd <cwd> && claude --resume <session_id>"`

#### Scenario: Click suppresses the stale entry briefly to avoid double flash
- **WHEN** the user clicks a stale entry
- **THEN** that entry SHALL be marked `resolving` and hidden from the UI for 10 seconds
- **AND** if a new entry with the same `session_id` arrives within those 10 seconds, the stale entry SHALL be removed from the board

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
- **AND** the `ConfigWatcher` SHALL pick up the change and push new values to the `VisibilityController` and adapters immediately without requiring an app restart

### Requirement: Hook installation via Claude Code plugin or Settings button

The system SHALL support two equivalent installation paths for the hook scripts: a Claude Code plugin and an in-app Settings button.

#### Scenario: Plugin installation registers hooks
- **WHEN** the user installs the `claude-pending-board` Claude Code plugin from the marketplace
- **THEN** the plugin's `plugin.json` SHALL register the three hooks (`Notification`, `UserPromptSubmit`, `Stop`) pointing to platform-appropriate scripts bundled inside the plugin
- **AND** no changes SHALL be made to the user's global `~/.claude/settings.json`

#### Scenario: Settings button edits global settings safely
- **WHEN** the user clicks "Install hooks" in the Settings window
- **THEN** the app SHALL first back up `~/.claude/settings.json` to `~/.claude/settings.json.pending-board-backup-<ts>`
- **AND** merge the three hook entries into the file preserving all existing entries
- **AND** show a confirmation diff before committing the write

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
- **WHEN** the Reaper triggers compaction (file > 5 MB or > 10 000 lines, or at startup)
- **THEN** the new content SHALL be written to `board.jsonl.tmp` and renamed over `board.jsonl` in a single step
- **AND** the watcher SHALL be paused during the rename to suppress spurious events
