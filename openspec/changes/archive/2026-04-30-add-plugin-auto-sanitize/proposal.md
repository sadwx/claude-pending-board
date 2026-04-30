# add-plugin-auto-sanitize

## Why

The bundled `plugin.json` ships one hook entry per OS for each event because Claude Code 2.1.x ignores the `platform` annotation on hook entries. Without intervention, `/hooks` lists every command (pwsh on macOS/Linux, bash on Windows) and Claude Code attempts to spawn each one — every fire ENOENTs on the wrong-OS commands, and `UserPromptSubmit` in particular shows three duplicate entries to the user.

The tray app already strips the foreign-platform entries from the on-disk `plugin.json` via `plugin_install::sanitize_installed_plugin_json`, but only at app boot and after a tray-driven `Install plugin` click. A user running `claude plugin update` (or being upgraded by marketplace auto-update) **while the tray app is already running** ends up with duplicate hook entries until they next restart the app or run the `--sanitize-manifest` CLI by hand. We saw this fire in practice on `0.2.4 → 0.2.5` upgrades.

Two adjacent issues should be addressed at the same time:

1. The existing sanitize behavior (boot-time + post-install + CLI flag) was never written into the working spec. Adding the new auto-update path is a good moment to backfill the requirement that covers all four trigger points.
2. The `INSTALL.md` troubleshooting section currently advises users to run the CLI flag manually after every update. With the watcher in place that becomes unnecessary for users who keep the tray app running.

## What Changes

- **MODIFIED** *Manifest sanitization* — adds a fourth trigger scenario: the tray app watches `~/.claude/plugins/cache/` recursively and re-runs the sanitizer (debounced 1.5 s) whenever a filesystem event mentions the `claude-pending-board` plugin path. Closes the live-update gap so `claude plugin update` lands a clean manifest without a tray-app restart.

## Out of scope

- **Removing per-OS entries from the source `plugin.json`.** That would require a single cross-platform launcher binary (or upstream honoring the `platform` field), both of which are bigger changes than this proposal targets.
- **Filing the upstream Claude Code bug.** Tracking issue is fine, but the workaround is needed regardless of when upstream fixes it.

## Capabilities

### Modified Capabilities

- `pending-board`: gains an explicit *Plugin manifest sanitization* requirement covering the four trigger points (boot, post-install, cache-change, CLI flag) and the watcher mechanism that makes auto-sanitize on update possible.
