# add-wsl-support

## Why

Linux support was dropped in commit `af26a1e` (2026-04-21) because the maintainer had no Linux device for release verification. The decision still holds for native Linux desktops, but it inadvertently cut off **WSL users running Claude Code inside Ubuntu** — a common configuration on Windows developer machines.

A PoC on `Ubuntu-24.04` has confirmed:

- The Windows tray app's file watcher reliably picks up writes that cross the WSL → Windows 9P boundary (~2–5 s latency).
- The existing bash hook fires on every `Notification` / `UserPromptSubmit` / `Stop` event in WSL Claude when registered manually, and clear ops match by `session_id` and remove entries cleanly even after the entry has been promoted to `stale`.
- Symlinking `~/.claude/pending` in WSL to `/mnt/c/Users/<winuser>/.claude/pending` is sufficient plumbing to share the board file between the two sides.

Three failure modes block a clean shipping experience today:

1. **Plugin manifest doesn't fire on Linux.** The `platform: "darwin"` gate on the bash hook means `claude plugin install …` inside WSL silently registers nothing.
2. **Reaper false-stales WSL entries.** It checks the **Windows** process table for `claude_pid`. WSL pids aren't there, so every WSL entry transitions to `stale` within the next 30-second sweep — entries that are actually live get the dim styling immediately.
3. **Click-to-focus is broken across the WSL boundary.** Clicking a stale WSL entry triggers `wezterm cli spawn` with the Linux `cwd` (e.g. `/home/user/project`) which Windows WezTerm cannot enter, and `claude --resume <id>` runs on Windows where the WSL session ID doesn't exist. The new tab dies on launch.

## What Changes

- **MODIFIED** *Live / stale liveness tracking* — entries with a `wsl_distro` field skip the Windows process-table check and remain live until cleared by a hook event, manual dismiss, or periodic stale cleanup.
- **MODIFIED** *Click to focus live terminal pane* — when the hook captured `$WEZTERM_PANE` the adapter activates that pane directly; otherwise falls back to the ancestor walk; WSL entries without a captured pane id still fall through to the resume path.
- **MODIFIED** *Click to resume stale entry* — for entries with a `wsl_distro` field, the WezTerm adapter translates the Linux `cwd` into a `\\wsl$\<distro>\…` UNC and runs `wsl.exe -d <distro> -e claude --resume <session_id>` instead of `claude --resume` directly on the Windows side.
- **ADDED** *WSL distro identification on board entries* — bash hook emits `"wsl_distro": "<name>"` on every op when `$WSL_DISTRO_NAME` is set; absent otherwise.
- **ADDED** *WezTerm pane identification on board entries* — both hook scripts emit `"wezterm_pane_id": "<id>"` when `$WEZTERM_PANE` is set, so click-to-focus can address an existing pane via `wezterm cli activate-pane` instead of guessing through the process tree.
- **ADDED** *Plugin manifest covers Linux platforms* — bash hook is registered for `platform: "linux"` in addition to `darwin`, so `claude plugin install` works inside WSL without manual `settings.json` editing. Reverses the relevant slice of `af26a1e`.
- **ADDED** *Automatic WSLENV configuration on Windows* — the tray app idempotently merges `WEZTERM_PANE/u` into the user's persistent `WSLENV` on every launch when WSL is detected, so click-to-focus works for WSL-origin entries with zero manual user setup. When the registry write happens and a `wezterm-gui.exe` process is already running, the HUD surfaces a one-shot "restart WezTerm" warning — WezTerm captures `WSLENV` at launch and never re-reads it, so the running instance has stale env until restarted.

## Out of scope

- **Native Linux desktop support.** This change targets WSL specifically. Re-introducing native Linux is a separate decision that depends on someone signing up to verify each release.
- **iTerm2 adapter and macOS behavior.** Unaffected.
- **WSL liveness via `wsl.exe -e ps -p <pid>`.** A correct-but-slow option for the reaper. We skip the check entirely for WSL entries; doing the cross-boundary `wsl ps` is deferred until we have a way to amortize the per-call cost.

## Capabilities

### Modified Capabilities

- `pending-board`: liveness tracking and click-to-focus now recognize WSL-origin entries and route them across the WSL/Windows boundary.
