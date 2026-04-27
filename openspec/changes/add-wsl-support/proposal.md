# add-wsl-support

## Why

Linux support was dropped in commit `af26a1e` (2026-04-21) because the maintainer had no Linux device for release verification. The decision is still defensible for native Linux desktops, but it inadvertently cut off **WSL users running Claude Code inside Ubuntu** — a common configuration on Windows developer machines.

A PoC on `Ubuntu-24.04` confirmed the broad strokes:

- The Windows tray app's file watcher reliably picks up writes that cross the WSL → Windows 9P boundary (~2–5 s latency, not zero, but acceptable).
- The existing bash hook fires on every `Notification` / `UserPromptSubmit` / `Stop` event in WSL Claude when registered manually; clear ops match by `session_id` and remove entries cleanly.
- Symlinking `~/.claude/pending` in WSL to `/mnt/c/Users/<winuser>/.claude/pending` is enough plumbing to share the board file between the two sides.

Three failure modes block a clean shipping experience today:

1. **Plugin manifest doesn't fire on Linux.** The `platform: "darwin"` gate on the bash hook means `claude plugin install …` inside WSL silently registers nothing — users have to hand-edit `~/.claude/settings.json`.
2. **Reaper false-stales WSL entries.** It checks the **Windows** process table for `claude_pid`. WSL pids aren't there, so every WSL entry is marked `pid_dead` within the next 30-second sweep. Visible as immediate stale styling on entries that are actually live.
3. **Click-to-focus is broken across the WSL boundary.** Stale-click triggers `wezterm cli spawn` with the Linux `cwd` (e.g. `/home/simon/project`) which Windows WezTerm cannot enter, and runs `claude --resume <id>` on Windows where the WSL session ID doesn't exist. The new tab dies on launch.

## What Changes

- **MODIFIED** `pending-board` capability — extend liveness tracking and click-to-focus behavior to recognize WSL-origin entries and route them correctly across the WSL/Windows boundary.
- **NEW** `wsl_distro` optional field on board entries, populated by the bash hook only when `$WSL_DISTRO_NAME` is set.
- **NEW** Reaper short-circuit: WSL-origin entries skip the Windows-side liveness check and remain live until cleared by a hook event (or by user dismiss).
- **MODIFIED** Plugin manifest — re-introduce `platform: "linux"` entries on the bash hook so `claude plugin install` from inside WSL registers the hooks correctly. (Reverses the relevant slice of `af26a1e`; native-Linux desktops remain unsupported, but the manifest no longer actively blocks WSL.)
- **MODIFIED** Bash hook script — emit `wsl_distro` when running inside WSL, otherwise omit the field (so macOS behavior is unchanged).
- **MODIFIED** WezTerm adapter — for entries with a `wsl_distro` field:
  - translate `cwd` from a Linux path to a `\\wsl$\<distro>\<path>` UNC for the spawn working directory;
  - launch the resume command as `wsl.exe -d <distro> -e claude --resume <session_id>` rather than calling `claude --resume` directly on the Windows side;
  - leave the focus-pane path unchanged (already pid-based and out-of-scope here).

## Out of scope

- **Native Linux desktop support.** This change is targeted at WSL specifically. The reaper/adapter changes don't help a Linux user without a Windows host. Re-introducing native Linux is a separate decision that depends on someone signing up to verify each release.
- **iTerm2 adapter.** macOS only; unaffected.
- **WSL liveness via `wsl.exe -e ps -p <pid>`.** A correct-but-slow option for the reaper. PR-A skips the check entirely; doing the cross-boundary `wsl ps` is deferred to a later iteration and tracked separately.

## Capabilities

### Modified Capabilities

- `pending-board`: liveness tracking and click-to-focus now recognize WSL-origin entries.
