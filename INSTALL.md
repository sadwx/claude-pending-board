# Installing Claude Pending Board

End-user install guide. This walks you through the full setup from zero to a working tray app.

> **Heads up**: the project is currently **pre-alpha** and no binaries are published yet. The steps below describe the install flow the v0.1.0 release will use. Until that release lands, only the "build from source" path in [Appendix A](#appendix-a-build-from-source-pre-alpha) works.

## Prerequisites

1. **Claude Code** installed. Verify with `claude --version`.
2. **A supported terminal** in your `PATH`:
   - **Windows**: [WezTerm](https://wezfurlong.org/wezterm/). Verify with `wezterm --version`.
   - **macOS**: WezTerm or [iTerm2](https://iterm2.com/).
3. **Write access** to `~/.claude/` and `~/.claude/pending/` (created automatically on first run).

Windows Terminal is not supported as a focus target (cannot programmatically activate a specific tab). You can still use it, but clicking a pending entry will not bring the right tab forward.

## Step 1 · Install the tray app

### Option A · From GitHub Releases (recommended)

1. Go to the [releases page](https://github.com/your-org/claude-pending-board/releases) and download the artifact for your OS:
   - Windows: `claude-pending-board-<version>-x64-setup.exe` or the portable `.msi`
   - macOS: `claude-pending-board-<version>.dmg` (universal)
2. Install / extract it to the usual place for your OS.
3. Launch it. A tray icon appears with a pink status dot.

### Option B · Via `winget` (Windows, after a stable release)

```powershell
winget install Suxi.ClaudePendingBoard
```

### Option C · Build from source

See [Appendix A](#appendix-a-build-from-source-pre-alpha).

## Step 2 · Wire Claude Code hooks

You have two equivalent install paths. Pick one.

### Path 1 · Claude Code plugin (recommended)

The plugin registers the hooks without editing your global `settings.json`.

```bash
/plugin marketplace add github:your-org/claude-pending-board
/plugin install claude-pending-board@claude-pending-board
/reload-plugins
```

Verify:

```bash
/pending-board doctor
```

You should see ✓ for: hook registration, script files, board file writable, terminal adapter in PATH.

### Path 2 · In-app Settings button

If you prefer to keep your hooks in your global `~/.claude/settings.json` rather than via a plugin:

1. Open the Claude Pending Board tray icon → **Settings…**
2. Click **Install hooks**.
3. The app will:
   - Back up your current `~/.claude/settings.json` to `~/.claude/settings.json.pending-board-backup-<timestamp>`
   - Show a diff preview of the three hook entries it will add (`Notification`, `UserPromptSubmit`, `Stop`)
   - Commit the merge once you confirm

To reverse:

1. Open **Settings…**
2. Click **Uninstall hooks**. Your backup file is left intact.

## Step 3 · First run walkthrough

1. **Start a Claude Code session in WezTerm / iTerm2**. Pick any project you have lying around.
2. **Trigger a permission prompt** — run a command that requires Claude to ask for Bash approval (e.g. "Run `ls`").
3. **The HUD auto-appears** near your tray icon with one red entry.
4. **Click the entry**. The WezTerm pane or iTerm2 session that owns it jumps to the foreground.
5. **Answer the prompt** in the terminal. The HUD auto-hides after a 2-second grace delay.

If any of these steps silently fail, jump to [Troubleshooting](#troubleshooting).

## Step 4 · Configure

Open **Settings…** from the tray and adjust any of the following:

| Setting | Default | Description |
|---|---|---|
| Cooldown after manual dismiss | 15 min | How long the HUD stays silent when you manually dismiss it |
| Reminding enabled | on | When on, the HUD re-opens at cooldown expiry if new items arrived during the cooldown |
| Auto-hide grace delay | 2 s | Delay after the last item clears before the HUD hides |
| Dismiss confirmation countdown | 5 s | Duration of the "Going silent for N minutes" panel |
| Skip dismiss confirmation | off | Bypass the confirmation panel entirely |
| Default terminal adapter | WezTerm (Windows) / iTerm2 (macOS) | Which adapter to use for focus and resume |
| HUD position | near tray | Drag the window to move; "Reset HUD position" returns it |

Changes apply immediately — no restart needed.

## Troubleshooting

### The HUD never appears when I trigger a permission prompt

1. Run `/pending-board doctor` (if you used the plugin).
2. Confirm `~/.claude/settings.json` has entries for `Notification`, `UserPromptSubmit`, and `Stop` hooks.
3. Tail `~/.claude/pending/logs/hook-errors.log` — if the hook fired but errored, the error lives here.
4. Tail `~/.claude/pending/board.jsonl` — if the hook wrote a line but the app didn't pick it up, restart the app.

### Clicking an entry doesn't focus the right pane

1. Make sure the right adapter is selected in Settings.
2. Verify the binary is in PATH: `wezterm --version` or check that iTerm2 is running on macOS.
3. On Windows, check Windows focus-steal protection — the HUD may flash the taskbar icon instead of stealing focus. Click the terminal icon to bring it forward.
4. If you run Claude Code inside an unsupported terminal (VS Code integrated terminal, Alacritty, ghostty, etc.), the ancestor walk won't find a known adapter and the click will fall through to `spawn_resume`.

### Entries never clear after I answer the prompt

The `UserPromptSubmit` hook isn't firing. Check `/pending-board doctor` and the contents of `~/.claude/pending/board.jsonl` — the `clear` op should appear within a second of your reply.

### The HUD appears at the wrong position on multi-monitor setups

If you unplug a monitor while a saved position is off-screen, the app resets to the tray-anchor default on next launch. If it doesn't, open **Settings… → Reset HUD position**.

### Logs

- Hook errors: `~/.claude/pending/logs/hook-errors.log`
- App logs: `~/.claude/pending/logs/app.log`
- Panic dumps: `~/.claude/pending/logs/panic.log`

Log verbosity defaults to `info`. Flip the "Debug logging" toggle in Settings for `trace`-level output.

## Uninstall

1. **Hooks**: in Settings, click **Uninstall hooks** (or remove the plugin with `/plugin uninstall claude-pending-board`).
2. **App**: uninstall via your OS package manager or drag to Trash / delete the folder.
3. **State files** (optional — if you want a fully clean slate): delete `~/.claude/pending/`.

## Appendix A · Build from source (pre-alpha)

Required:

- Rust 1.83 or later (`rustup update`)
- Tauri 2 [prerequisites for your OS](https://v2.tauri.app/start/prerequisites/)
- Node.js 20+ (for the front-end build step)

Steps:

```bash
git clone https://github.com/your-org/claude-pending-board
cd claude-pending-board
cargo tauri build
```

The built binary lands in `crates/app/src-tauri/target/release/bundle/`. Copy it to a stable location and launch.

To run in dev mode with hot reload:

```bash
cargo tauri dev
```

Hook scripts live in `scripts/` and can be invoked directly while iterating on them — pipe a sample JSON payload into the script and check `~/.claude/pending/board.jsonl`:

```powershell
# Windows
'{"hook_event_name":"Notification","session_id":"...","cwd":"...","transcript_path":"...","notification_type":"permission_prompt","message":"Test"}' | pwsh -File scripts/pending_hook.ps1
```

```bash
# macOS
echo '{"hook_event_name":"Notification","session_id":"...","cwd":"...","transcript_path":"...","notification_type":"permission_prompt","message":"Test"}' | bash scripts/pending_hook.sh
```
