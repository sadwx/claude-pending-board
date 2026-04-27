# pending-board

## MODIFIED Requirements

### Requirement: Live / stale liveness tracking

The system SHALL continuously verify that every live entry on the board corresponds to a still-running Claude Code process, and promote dead entries to the `stale` state. Entries that originated inside WSL SHALL be treated as live without consulting the Windows process table, because their `claude_pid` belongs to a different OS namespace.

#### Scenario: Claude Code process still alive

- **WHEN** the Reaper runs its periodic check (every 30 seconds) on a live entry with `claude_pid = P` and `wsl_distro = None`
- **AND** process `P` exists in the OS process table
- **AND** `~/.claude/sessions/P.json` exists with a `sessionId` matching the entry's `session_id`
- **THEN** the entry SHALL remain in the `live` state

#### Scenario: Claude Code process is dead

- **WHEN** the Reaper runs on a live entry with `wsl_distro = None`
- **AND** process `claude_pid` no longer exists in the OS process table
- **THEN** the entry SHALL be promoted to the `stale` state with `reason = "pid_dead"` and the periodic stale cleanup SHALL eventually drop it

#### Scenario: WSL-origin entry is trusted live

- **WHEN** the Reaper runs on a live entry with `wsl_distro = Some(<distro_name>)`
- **THEN** the Reaper SHALL skip the Windows-side process and session-file checks for that entry
- **AND** the entry SHALL remain `live` until the Claude Code session in WSL emits a clearing op (`UserPromptSubmit` or `Stop`), the user dismisses it manually, or the periodic stale cleanup loop expires it after the configured TTL

### Requirement: Click-to-focus and click-to-resume

The system SHALL focus the owning terminal pane when the user clicks a live entry, and SHALL spawn a new tab running `claude --resume <session_id>` when the user clicks a stale entry. For entries that originated inside WSL, the resume path SHALL launch Claude inside the originating WSL distro and SHALL set the new tab's working directory to the corresponding `\\wsl$\<distro>\<linux-cwd>` UNC path so the tab opens at the right project.

#### Scenario: Live entry click on a native (non-WSL) session

- **WHEN** the user clicks a live entry with `wsl_distro = None`
- **AND** the configured terminal adapter can identify the owning pane via `terminal_pid`
- **THEN** the adapter SHALL focus that pane and bring its window to the foreground

#### Scenario: Stale entry click on a native (non-WSL) session

- **WHEN** the user clicks a stale entry with `wsl_distro = None`
- **THEN** the adapter SHALL spawn a new tab in its terminal of choice running `claude --resume <session_id>` with `cwd = entry.cwd`

#### Scenario: Stale entry click on a WSL-origin session

- **WHEN** the user clicks a stale entry with `wsl_distro = Some("Ubuntu-24.04")` and `cwd = "/home/simon/project"`
- **THEN** the adapter SHALL spawn a new WezTerm tab with working directory `\\wsl$\Ubuntu-24.04\home\simon\project`
- **AND** the tab SHALL run `wsl.exe -d Ubuntu-24.04 -e claude --resume <session_id>` so the resumed Claude session lives inside the originating distro

#### Scenario: Live entry click on a WSL-origin session

- **WHEN** the user clicks a live entry with `wsl_distro = Some(<distro>)`
- **THEN** the click SHALL be treated identically to the stale path above, because the `terminal_pid` carried on a WSL-origin entry refers to a process inside WSL and cannot be focused by the Windows-side adapter

## ADDED Requirements

### Requirement: WSL distro identification on board entries

The system SHALL record the originating WSL distro on every entry produced by a Claude Code session running inside WSL, so downstream consumers (reaper, adapters) can route the entry correctly across the WSL/Windows boundary.

#### Scenario: Hook fires inside WSL

- **WHEN** the bash hook script (`pending_hook.sh`) handles a `Notification` event
- **AND** the environment variable `WSL_DISTRO_NAME` is non-empty
- **THEN** the appended `add` op SHALL include a string field `"wsl_distro": "<name>"` matching the value of `$WSL_DISTRO_NAME`

#### Scenario: Hook fires on macOS or native Linux

- **WHEN** the bash hook script handles a `Notification` event
- **AND** `WSL_DISTRO_NAME` is unset or empty
- **THEN** the `add` op SHALL omit the `wsl_distro` field entirely (not write `null`, not write an empty string)

### Requirement: Plugin manifest covers Linux platforms

The Claude Code plugin SHALL register its hook script for the `linux` platform in addition to `windows` and `darwin`, so that running `claude plugin install claude-pending-board@claude-pending-board` inside WSL registers all three hooks without manual `settings.json` editing.

#### Scenario: Plugin install from inside WSL

- **WHEN** a user runs `claude plugin marketplace add sadwx/claude-pending-board` followed by `claude plugin install claude-pending-board@claude-pending-board` inside a WSL distro
- **THEN** Claude Code SHALL register the bash variant of `pending_hook.sh` for the `Notification`, `UserPromptSubmit`, and `Stop` events under the user's `~/.claude/settings.json` (or its plugin equivalent)
- **AND** subsequent Claude sessions inside that WSL distro SHALL fire the hook on every event without further configuration
