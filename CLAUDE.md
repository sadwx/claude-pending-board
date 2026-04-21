# CLAUDE.md

Project-specific context for Claude Code sessions working on this repo.

## What this is

`claude-pending-board` is a cross-platform tray app that surfaces every waiting Claude Code CLI session in one floating HUD. Claude Code hooks write to `~/.claude/pending/board.jsonl`; the tray app watches that file and renders a HUD, and clicking an entry focuses the owning WezTerm (Windows) or iTerm2 (macOS) pane.

Design + spec live under `openspec/changes/add-claude-pending-board/`.

## Repo layout

```
claude-pending-board/
├── crates/
│   ├── core/        # pure Rust, no Tauri — parser, store, watcher,
│   │                # compaction, visibility FSM, reaper, config,
│   │                # terminal trait + ancestor walk
│   ├── adapters/    # WezTerm (all OSes) + iTerm2 (macOS cfg-gated)
│   │                # implementations of the terminal trait
│   └── app/         # Tauri 2 app — commands, tray, services,
│                    # HUD window, Settings window
├── scripts/         # source of truth for hook scripts
│                    # (PowerShell + Bash), plus smoke tests
├── plugin/          # Claude Code plugin — copies hook scripts into
│                    # its own hooks/ and registers them via
│                    # .claude-plugin/plugin.json
├── openspec/        # change proposals, design, spec, tasks
├── docs/
│   ├── superpowers/plans/   # phase implementation plans
│   ├── release-checklist.md # manual verification before tagging
│   └── screenshots/         # comparison with v5 design mock
└── .github/workflows/       # ci.yml (fmt+clippy+test) + release.yml
```

## Commands

```bash
# Build
cargo build -p claude-pending-board-app                # debug, Windows
cargo tauri build                                       # release, current OS

# Run
./target/debug/claude-pending-board-app.exe             # directly
cargo tauri dev                                         # with dev tools / hot reload

# Test
cargo test --workspace                                  # all 66 tests
cargo test -p claude-pending-board-core -- parser       # filter by module
cargo test -- --ignored                                 # contract tests (requires WezTerm)

# Lint / format — MUST pass before committing
cargo fmt --check --all
cargo clippy --workspace -- -D warnings

# Smoke tests
pwsh scripts/smoke-test-auto.ps1                        # ~40s non-interactive
pwsh scripts/smoke-test.ps1                             # interactive, for humans
bash scripts/smoke-test.sh                              # same, for macOS
```

## Conventions

- **TDD with unit tests inline at the bottom of each module** in a `#[cfg(test)] mod tests` block. Integration tests go in `crates/core/tests/`.
- **Catppuccin Mocha** palette for all UI (colors defined as CSS custom properties in `crates/app/ui/hud/style.css`).
- **Vanilla HTML/CSS/JS** on the frontend — no framework, no bundler. Script tags use `defer` or load at bottom; avoid `type="module"` unless actually importing.
- **No `innerHTML` with dynamic content** — the security hook blocks it. Use `textContent` or build with `createElement`, or use separate hidden elements and swap visibility.
- **Windows line endings** (CRLF) are auto-applied on checkout — don't fight the warnings.
- **Commits are conventional-ish**: `feat(scope):`, `fix(scope):`, `docs:`, `test:`, `ci:`, `chore:`. Scope is usually the crate name (`core`, `adapters`, `app`, `plugin`, `hooks`).

## Known gotchas (things that bit us)

- **`sysinfo 0.33` API**: use `ProcessRefreshKind::everything()`, not `::new()` (which no longer exists). Same for `ProcessesToUpdate::All`.
- **Tauri window creation from a command handler hangs WebView2 on Windows.** Pre-create all windows during `tauri::Builder::setup()` on the main thread with `visible(false)`, then show/hide them from commands. This is why `hud` and `settings` are both created in `crates/app/src/main.rs` at boot.
- **Tauri 2 capabilities must exist for `invoke`/`listen` to work.** Without `crates/app/capabilities/default.json` listing the window labels and granting `core:default`, the JS bridge silently fails and the window renders blank.
- **`frontendDist` is relative to `tauri.conf.json`** and paths can't use `..`. Current config points `frontendDist: "ui"` and windows load `hud/index.html` or `settings/index.html`.
- **Settings window: intercept close-to-hide.** Default behavior destroys the window, which breaks the pre-create strategy. The close request handler in `main.rs` calls `api.prevent_close()` + `window.hide()`.
- **Hook scripts must always `exit 0`.** Any non-zero exit blocks Claude Code. Both `pending_hook.ps1` and `pending_hook.sh` wrap their bodies in `try`/`catch` and log to `~/.claude/pending/logs/hook-errors.log` on failure.
- **Pill and countdown positioning**: the DEFAULT pill is `position: absolute; top: -10px; left: 50%; transform: translateX(-50%)` — floats above the button border, not inline after the label. The countdown is an inline `<span>` sibling to `.btn-label` so "Wake me · 5s" renders on one line.
- **HUD width must be `100%` not `380px`**. DPI scaling can make explicit pixel widths clip the dismiss X button. The window size (`inner_size(380.0, 440.0)`) comes from Tauri; CSS fills it.
- **Tray left-click needs `show_menu_on_left_click(false)`.** Tauri 2's default is to show the attached menu on any click, so the custom `on_tray_icon_event` for `MouseButton::Left` never fires. Without this, left-click just opens the menu — breaking the "left-click re-opens HUD, right-click shows menu" UX. Set it explicitly on the `TrayIconBuilder` in `crates/app/src/tray.rs`.
- **HUD drag on macOS needs both the explicit capability and a JS fallback.** `core:default` does NOT include `core:window:allow-start-dragging` — the capability must be listed explicitly in `crates/app/capabilities/default.json`. On top of that, `data-tauri-drag-region` alone was unreliable on macOS with `decorations(false) + always_on_top`; a manual `mousedown → getCurrentWindow().startDragging()` handler on `.header` in `crates/app/ui/hud/main.js` works consistently. Keep both.
- **Icon must be RGBA on macOS.** `crates/app/icons/icon.png` must have an alpha channel; plain 8-bit RGB fails at compile time with `icon ... is not RGBA` from `tauri::generate_context!`. If regenerating the icon, run `python3 -c "from PIL import Image; Image.open('icon.png').convert('RGBA').save('icon.png')"` (or equivalent) before committing.

## Don't edit

- `crates/app/gen/` — regenerated by `tauri-build` on every build. Changes are lost.
- Both copies of the hook scripts in sync: `scripts/pending_hook.*` is the source of truth, and `plugin/hooks/pending_hook.*` is a copy. If you change one, copy to the other. Or better, make the plugin build step sync them automatically (not yet done).

## Phase plans (for historical context)

- Phase 1 — core library: `docs/superpowers/plans/2026-04-16-phase1-core-library.md`
- Phase 2 — adapters + hook scripts: `docs/superpowers/plans/2026-04-16-phase2-adapters-hooks.md`
- Phase 3 — Tauri app + UI: `docs/superpowers/plans/2026-04-16-phase3-tauri-app.md`
- Phase 4 — plugin + docs + CI + release: `docs/superpowers/plans/2026-04-17-phase4-plugin-docs-release.md`

## Before cutting a release

Run through `docs/release-checklist.md` fully, especially the manual UI scenarios that `smoke-test-auto.ps1` cannot cover.
