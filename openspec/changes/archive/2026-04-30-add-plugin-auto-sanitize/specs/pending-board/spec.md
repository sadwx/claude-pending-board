# pending-board (auto-sanitize delta)

This change document expresses the deltas that `add-plugin-auto-sanitize` introduces on top of the working spec at `openspec/specs/pending-board/spec.md`.

## MODIFIED Requirements

### Requirement: Manifest sanitization

The tray app SHALL strip foreign-platform hook entries from the installed `plugin.json` so the user's `/hooks` listing only shows entries that can run on the current OS. The shipped `plugin.json` carries one entry per OS for each event because Claude Code 2.1.x ignores the `platform` field; this requirement keeps the runtime view tidy without depending on Claude Code honoring `platform`. While the tray app is running, the sanitize SHALL also re-fire automatically whenever the plugin cache changes on disk, so `claude plugin update` (or marketplace auto-update) does not leave duplicate entries lingering until the next app reboot.

#### Scenario: Sanitize after one-click install from the setup card

- **WHEN** `install_plugin` succeeds via the setup card path
- **THEN** the tray app SHALL rewrite `~/.claude/plugins/cache/<marketplace>/<plugin>/<version>/.claude-plugin/plugin.json` to remove every entry whose `platform` field is set and does not match the current OS (`windows`, `darwin`, or `linux`)
- **AND** entries with no `platform` field SHALL be preserved unchanged
- **AND** sanitize failure SHALL be logged at `warn` level and SHALL NOT fail the install

#### Scenario: Sanitize on tray-app startup

- **WHEN** the tray app boots
- **THEN** the app SHALL run the same sanitize as a best-effort background task during `setup()`, so a Claude-Code-driven plugin auto-update that landed since the last app launch is cleaned up before the user opens `/hooks`

#### Scenario: Sanitize on demand via CLI flag

- **WHEN** the user invokes the binary as `claude-pending-board-app --sanitize-manifest`
- **THEN** the binary SHALL run sanitize without booting Tauri, print `removed N foreign-platform hook entries from plugin.json.` (or `plugin.json already clean — no foreign-platform entries.` when N = 0) to stderr, and exit with status 0
- **AND** SHALL exit with status 1 and a `sanitize failed: <reason>` message on error
- **AND** the operation SHALL be idempotent: re-running on an already-clean manifest SHALL report `already clean`

#### Scenario: Auto-sanitize on plugin cache change

- **WHEN** the tray app is running and any filesystem event under `~/.claude/plugins/cache/` mentions a path component named `claude-pending-board` (typical of `claude plugin install` / `claude plugin update` / marketplace auto-update creating or replacing a version directory)
- **THEN** the app SHALL coalesce the burst of events through a debounce window (1.5 s, chosen so a single install settles inside it but stale duplicates clear within ~2 s of the install completing)
- **AND** SHALL run the sanitize routine once per debounced burst
- **AND** filesystem events that touch other plugins' cache subdirectories SHALL NOT trigger a sanitize call
- **AND** sanitize failure SHALL be logged at `warn` level only and SHALL NOT propagate further
