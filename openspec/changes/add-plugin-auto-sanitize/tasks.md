# Tasks — Plugin manifest auto-sanitize

Single-PR change. The implementation is contained to the tray app (no core/adapters changes, no hook-script changes).

## Implementation

- [x] New module `crates/app/src/plugin_watch.rs` exposing `PluginCacheWatcher::start_default()` returning a held watcher value. Internally uses `notify-debouncer-full` with a 1.5 s window.
- [x] Filter logic: only fire the callback when the debounced batch contains at least one `Create` / `Modify` / `Remove` event whose path components include `"claude-pending-board"`. Other plugins' cache activity must not trigger sanitize.
- [x] Wire into `services::boot()`: build the closure that invokes `plugin_install::sanitize_installed_plugin_json()` (already idempotent), start the watcher, `mem::forget` it for the app lifetime. Errors are logged at WARN, not surfaced to the user.
- [x] Add `notify` and `notify-debouncer-full` to `crates/app/Cargo.toml` (already declared in `[workspace.dependencies]`).
- [x] Declare `mod plugin_watch;` in `crates/app/src/main.rs`.

## Tests

- [x] Unit: `fires_on_plugin_subdir_creation` — start watcher on a tempdir, create `<tempdir>/claude-pending-board/claude-pending-board/<v>/.claude-plugin/plugin.json`, assert callback fires within debounce + slack.
- [x] Unit: `ignores_changes_outside_our_plugin` — start watcher, mutate a sibling subdir named `some-other-plugin`, assert callback never fires.
- [x] Unit: `coalesces_burst_into_single_call` — write 20 files within the debounce window, assert the callback fires 1–2 times (never per-file).

## Spec

- [x] Add `Plugin manifest sanitization` requirement to working spec via `specs/pending-board/spec.md` delta.

## Docs

- [x] `CLAUDE.md`: add a gotcha entry about the watcher; existing "Plugin versioning" section already references sanitize at boot time.
- [x] `INSTALL.md`: tone down the manual `--sanitize-manifest` step — note that it's a fallback for users who don't keep the tray app running; for everyone else the watcher handles it automatically.

## Validation

- [x] `cargo fmt --check --all`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `cargo test --workspace`
- [x] Manual: with tray app running, `claude plugin install` then check `~/.claude/plugins/cache/.../plugin.json` — should have only the current platform's hook entries within ~2 s of the install completing.
