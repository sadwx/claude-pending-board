# Installing Claude Pending Board

End-user install guide.

> **Heads up**: the project is currently **alpha**. The first tagged release is `v0.1.0`, flagged as a pre-release on GitHub. Binaries are unsigned, so Windows SmartScreen and macOS Gatekeeper will warn on install — see the platform notes below.

## Prerequisites

1. **Claude Code** installed and in `PATH`. Verify with `claude --version`.
2. **A supported terminal**:
   - **Windows**: [WezTerm](https://wezfurlong.org/wezterm/). Verify with `wezterm --version`.
   - **macOS**: WezTerm or [iTerm2](https://iterm2.com/).
3. **Write access** to `~/.claude/` (both the tray app and Claude Code write under this directory; created automatically on first run).

Windows Terminal is not supported as a focus target (cannot programmatically activate a specific tab). You can still run Claude Code inside Windows Terminal; clicking a pending entry just won't focus the right tab.

## Two-step install

### Step 1 · Install the tray app

Download the artifact for your OS from the [releases page](https://github.com/sadwx/claude-pending-board/releases):

| OS | File | Notes |
|---|---|---|
| Windows | `Claude.Pending.Board_<version>_x64_en-US.msi` | MSI installer. Double-click, approve UAC. |
| Windows | `Claude.Pending.Board_<version>_x64-setup.exe` | NSIS portable-style installer. |
| macOS | `Claude.Pending.Board_<version>_universal.dmg` | Drag to Applications. |

**Windows**: SmartScreen may warn "Windows protected your PC". Click *More info → Run anyway*. This is expected because the artifact is unsigned during alpha.

**macOS**: Gatekeeper may say "can't be opened". Right-click the app → *Open*, or run `xattr -dr com.apple.quarantine /Applications/Claude\ Pending\ Board.app` once to clear the quarantine attribute.

After install, launch the app. A Catppuccin-pink "C" icon appears in the tray.

### Step 2 · Install the Claude Code plugin (hooks)

The tray app won't see any sessions until the plugin is registered. The easiest path: **click the tray icon → [Install plugin]** in the HUD's first-run setup card. The app shells out to the `claude plugin` CLI under your user account.

If you prefer the CLI directly:

```bash
claude plugin marketplace add sadwx/claude-pending-board
claude plugin install claude-pending-board@claude-pending-board
```

Or from any Claude Code session:

```
/plugin marketplace add github:sadwx/claude-pending-board
/plugin install claude-pending-board@claude-pending-board
/reload-plugins
```

Any of the three paths produces the same result: three hooks (`Notification`, `UserPromptSubmit`, `Stop`) registered with Claude Code.

### Step 2.5 · WSL (Windows users running Claude Code inside WSL)

If you launch Claude Code inside WSL via WezTerm, install the plugin **from inside WSL**, not from a native Windows shell. The Linux hook script is what fires there. Open a WSL tab and run:

```bash
claude plugin marketplace add sadwx/claude-pending-board
claude plugin install claude-pending-board@claude-pending-board
```

Then — **once per Windows user** — let `WEZTERM_PANE` cross the Windows→WSL boundary so click-to-focus can address the right tab. From a Windows PowerShell tab:

```powershell
setx WSLENV "$env:WSLENV;WEZTERM_PANE/u"
```

Open a fresh WezTerm tab afterward. Verify with `echo $WEZTERM_PANE` inside WSL — it should print a number.

Without this step, WSL entries still appear in the HUD and clicking still resumes the session, but it opens a fresh tab instead of focusing the existing one.

The Windows tray app is what runs and renders the HUD; only the hook scripts live in WSL.

## Step 3 · Verify

1. **Start a Claude Code session in WezTerm / iTerm2** in any project.
2. **Trigger a permission prompt** — e.g. ask Claude to run `ls`.
3. **The HUD auto-appears** near your tray icon with one red entry.
4. **Click the entry**. The WezTerm pane or iTerm2 session that owns it jumps to the foreground.
5. **Answer the prompt** in the terminal. The HUD auto-hides after a ~2-second grace delay.

If any step silently fails, jump to [Troubleshooting](#troubleshooting).

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

1. Reopen the HUD (tray left-click). If it shows the "Hooks not installed" setup card, the plugin step was skipped — run the install from the card or the CLI commands above.
2. Check the plugin list: `claude plugin list` should include `claude-pending-board`.
3. Tail `~/.claude/pending/logs/hook-errors.log` — if a hook fired but errored, the error lives here.
4. Tail `~/.claude/pending/board.jsonl` — if the hook wrote a line but the app didn't pick it up, restart the app.

### The setup card says "Claude Code not found"

The tray app couldn't find the `claude` CLI in `PATH`. Install Claude Code first, then restart the tray app (or reopen the HUD for it to re-check).

### Clicking an entry doesn't focus the right pane

1. Verify the right adapter is selected in Settings.
2. Verify the binary is in PATH: `wezterm --version` or check that iTerm2 is running on macOS.
3. On Windows, check Windows focus-steal protection — the HUD may flash the taskbar icon instead of stealing focus. Click the terminal icon to bring it forward.
4. If you run Claude Code inside an unsupported terminal (VS Code integrated terminal, Alacritty, ghostty, etc.), the ancestor walk won't find a known adapter and the click will fall through to `spawn_resume`.

### Entries never clear after I answer the prompt

The `UserPromptSubmit` hook isn't firing. Check `claude plugin list` and the contents of `~/.claude/pending/board.jsonl` — the `clear` op should appear within a second of your reply.

### The HUD appears at the wrong position on multi-monitor setups

If you unplug a monitor while a saved position is off-screen, the app resets to the tray-anchor default on next launch. If it doesn't, open **Settings… → Reset HUD position**.

### Logs

- Hook errors: `~/.claude/pending/logs/hook-errors.log`
- App logs: `~/.claude/pending/logs/app.log`
- Panic dumps: `~/.claude/pending/logs/panic.log`

Log verbosity defaults to `info`. Flip the "Debug logging" toggle in Settings for `trace`-level output.

## Uninstall

1. **Plugin** (hooks):
   ```bash
   claude plugin uninstall claude-pending-board
   ```
   or from inside a Claude session: `/plugin uninstall claude-pending-board`.
2. **Tray app**:
   - Windows: Settings → Apps → uninstall "Claude Pending Board".
   - macOS: drag `/Applications/Claude Pending Board.app` to Trash.
3. **State files** (optional — if you want a fully clean slate): delete `~/.claude/pending/`.

## Appendix A · Build from source

Required:

- Rust 1.83 or later (`rustup update`)
- Tauri 2 [prerequisites for your OS](https://v2.tauri.app/start/prerequisites/)
- Node.js 20+ (for the front-end build step)

Steps:

```bash
git clone https://github.com/sadwx/claude-pending-board
cd claude-pending-board
cargo tauri build
```

The built binary lands under `target/release/bundle/`. Copy it to a stable location and launch.

To run in dev mode with hot reload:

```bash
cargo tauri dev
```

Hook scripts live in `scripts/` and can be invoked directly while iterating — pipe a sample JSON payload into the script and check `~/.claude/pending/board.jsonl`:

```powershell
# Windows
'{"hook_event_name":"Notification","session_id":"...","cwd":"...","transcript_path":"...","notification_type":"permission_prompt","message":"Test"}' | pwsh -File scripts/pending_hook.ps1
```

```bash
# macOS
echo '{"hook_event_name":"Notification","session_id":"...","cwd":"...","transcript_path":"...","notification_type":"permission_prompt","message":"Test"}' | bash scripts/pending_hook.sh
```
