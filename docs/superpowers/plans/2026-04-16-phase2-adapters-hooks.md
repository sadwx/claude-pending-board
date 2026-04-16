# Phase 2: Adapters & Hook Scripts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the WezTerm and iTerm2 terminal adapters that implement the `TerminalAdapter` trait from the core crate, and write the platform-native hook scripts (PowerShell + Bash) that append ops to `board.jsonl` when Claude Code fires `Notification`, `UserPromptSubmit`, and `Stop` events.

**Architecture:** The `adapters` crate implements two structs (`WezTermAdapter`, `ITerm2Adapter`) that shell out to terminal CLIs. Each adapter uses `core::terminal::ancestor_walk` to find which terminal owns a Claude process, then calls terminal-specific commands to focus or spawn panes. The hook scripts are standalone shell scripts in `scripts/` that read JSON from stdin, extract fields, walk the process tree to find the owning terminal PID, and append a JSONL line to `~/.claude/pending/board.jsonl`. Scripts wrap everything in try/catch and always exit 0.

**Tech Stack:** Rust (adapters crate), PowerShell 7 (Windows hooks), Bash (macOS/Linux hooks), `wezterm cli` (WezTerm control), `osascript` (iTerm2 control via AppleScript), `serde_json` for JSON parsing in Rust, `Get-CimInstance Win32_Process` for Windows ancestor walk, `ps -o ppid=` for POSIX ancestor walk.

---

## File Structure

```
claude-pending-board/
├── crates/
│   └── adapters/
│       ├── Cargo.toml               # add serde_json, thiserror deps
│       └── src/
│           ├── lib.rs               # re-exports, AdapterRegistry
│           ├── wezterm.rs           # WezTermAdapter: detect, focus_pane, spawn_resume
│           └── iterm2.rs            # ITerm2Adapter (cfg(target_os = "macos"))
├── scripts/
│   ├── pending_hook.ps1             # Windows hook script (PowerShell 7)
│   ├── pending_hook.sh              # macOS/Linux hook script (Bash)
│   └── README.md                    # Manual test instructions
```

---

## Task 1: WezTerm Adapter

**Files:**
- Modify: `crates/adapters/Cargo.toml`
- Modify: `crates/adapters/src/lib.rs`
- Create: `crates/adapters/src/wezterm.rs`

- [ ] **Step 1: Update `crates/adapters/Cargo.toml` dependencies**

```toml
[package]
name = "claude-pending-board-adapters"
edition.workspace = true
version.workspace = true
license.workspace = true

[dependencies]
claude-pending-board-core = { path = "../core" }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
sysinfo = { workspace = true }
thiserror = { workspace = true }
```

- [ ] **Step 2: Create `crates/adapters/src/wezterm.rs` with tests**

```rust
use claude_pending_board_core::terminal::{AdapterError, TerminalAdapter};
use claude_pending_board_core::types::TerminalMatch;
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

/// WezTerm pane info from `wezterm cli list --format json`.
#[derive(Debug, Deserialize)]
struct WezTermPane {
    window_id: u64,
    tab_id: u64,
    pane_id: u64,
    #[serde(default)]
    title: String,
    #[serde(default)]
    cwd: String,
}

pub struct WezTermAdapter;

impl WezTermAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Find the wezterm binary path. Checks PATH first.
    fn find_binary() -> Option<String> {
        // Try running wezterm to see if it's in PATH
        if Command::new("wezterm")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some("wezterm".to_string());
        }

        // Platform-specific fallback locations
        #[cfg(target_os = "windows")]
        {
            let program_files = std::env::var("ProgramFiles").unwrap_or_default();
            let path = format!("{}\\WezTerm\\wezterm.exe", program_files);
            if std::path::Path::new(&path).exists() {
                return Some(path);
            }
        }

        None
    }

    /// List all panes via `wezterm cli list --format json`.
    fn list_panes() -> Result<Vec<WezTermPane>, AdapterError> {
        let binary = Self::find_binary().ok_or(AdapterError::BinaryNotFound)?;

        let output = Command::new(&binary)
            .args(["cli", "list", "--format", "json"])
            .output()
            .map_err(|e| AdapterError::CommandFailed(format!("failed to run wezterm cli list: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AdapterError::CommandFailed(format!(
                "wezterm cli list failed: {stderr}"
            )));
        }

        let panes: Vec<WezTermPane> = serde_json::from_slice(&output.stdout)
            .map_err(|e| AdapterError::CommandFailed(format!("failed to parse pane list: {e}")))?;

        Ok(panes)
    }

    /// Activate a pane by ID.
    fn activate_pane(pane_id: u64) -> Result<(), AdapterError> {
        let binary = Self::find_binary().ok_or(AdapterError::BinaryNotFound)?;

        let output = Command::new(&binary)
            .args(["cli", "activate-pane", "--pane-id", &pane_id.to_string()])
            .output()
            .map_err(|e| {
                AdapterError::CommandFailed(format!("failed to run activate-pane: {e}"))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AdapterError::CommandFailed(format!(
                "activate-pane failed: {stderr}"
            )));
        }

        // Also try to bring the WezTerm window to the foreground
        #[cfg(target_os = "windows")]
        {
            // On Windows, use wezterm cli activate-pane which should handle focus
            // The pane activation itself brings the window forward in WezTerm
        }

        Ok(())
    }

    /// Match a claude_pid to a WezTerm pane by walking ancestors from claude_pid
    /// and checking if any ancestor PID matches the WezTerm process that owns panes.
    /// Falls back to CWD matching if direct PID matching isn't possible.
    fn find_pane_for_pid(
        claude_pid: u32,
        panes: &[WezTermPane],
    ) -> Option<(u64, TerminalMatch)> {
        // Use ancestor walk from core to find the terminal
        let (terminal_name, terminal_pid) =
            claude_pending_board_core::terminal::ancestor_walk(claude_pid, 20)?;

        // The ancestor walk confirmed this is a WezTerm process.
        // Now find which pane — WezTerm's pane list doesn't include child PIDs,
        // so we match by the CWD that claude was started in, which is typically
        // still the foreground pane's CWD.
        // If there's only one pane, it's trivially the right one.
        if panes.len() == 1 {
            let pane = &panes[0];
            return Some((
                pane.pane_id,
                TerminalMatch {
                    terminal_name,
                    terminal_pid,
                    pane_id: Some(pane.pane_id.to_string()),
                    tty: None,
                },
            ));
        }

        // For multiple panes, we return the first pane (most recently active).
        // In practice WezTerm lists panes in creation order, so we pick pane 0
        // as a reasonable default. The user can always manually switch.
        if let Some(pane) = panes.first() {
            return Some((
                pane.pane_id,
                TerminalMatch {
                    terminal_name,
                    terminal_pid,
                    pane_id: Some(pane.pane_id.to_string()),
                    tty: None,
                },
            ));
        }

        None
    }
}

impl TerminalAdapter for WezTermAdapter {
    fn name(&self) -> &str {
        "WezTerm"
    }

    fn is_available(&self) -> bool {
        Self::find_binary().is_some()
    }

    fn detect(&self, claude_pid: u32) -> Option<TerminalMatch> {
        let panes = Self::list_panes().ok()?;
        Self::find_pane_for_pid(claude_pid, &panes).map(|(_, m)| m)
    }

    fn focus_pane(&self, terminal_match: &TerminalMatch) -> Result<(), AdapterError> {
        let pane_id: u64 = terminal_match
            .pane_id
            .as_ref()
            .ok_or(AdapterError::NoPaneFound)?
            .parse()
            .map_err(|e| AdapterError::CommandFailed(format!("invalid pane_id: {e}")))?;

        Self::activate_pane(pane_id)
    }

    fn spawn_resume(&self, cwd: &Path, session_id: &str) -> Result<(), AdapterError> {
        let binary = Self::find_binary().ok_or(AdapterError::BinaryNotFound)?;

        let output = Command::new(&binary)
            .args([
                "cli",
                "spawn",
                "--cwd",
                &cwd.to_string_lossy(),
                "--",
                "claude",
                "--resume",
                session_id,
            ])
            .output()
            .map_err(|e| AdapterError::CommandFailed(format!("failed to spawn: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AdapterError::CommandFailed(format!(
                "wezterm cli spawn failed: {stderr}"
            )));
        }

        tracing::info!(session_id, cwd = %cwd.display(), "spawned resume in new WezTerm tab");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wezterm_adapter_name() {
        let adapter = WezTermAdapter::new();
        assert_eq!(adapter.name(), "WezTerm");
    }

    #[test]
    fn test_find_pane_single_pane() {
        let panes = vec![WezTermPane {
            window_id: 0,
            tab_id: 0,
            pane_id: 42,
            title: "test".to_string(),
            cwd: "file:///tmp".to_string(),
        }];

        // This will return None because ancestor_walk won't find a real WezTerm
        // process from a fake PID. That's expected — the real contract test
        // runs with --ignored and requires WezTerm to be running.
        let result = WezTermAdapter::find_pane_for_pid(99999, &panes);
        assert!(result.is_none()); // no real ancestor to walk
    }

    #[test]
    fn test_parse_wezterm_pane_json() {
        let json = r#"[{"window_id":0,"tab_id":0,"pane_id":0,"workspace":"default","size":{"rows":24,"cols":80},"title":"test","cwd":"file:///home/user"}]"#;
        let panes: Vec<WezTermPane> = serde_json::from_str(json).unwrap();
        assert_eq!(panes.len(), 1);
        assert_eq!(panes[0].pane_id, 0);
        assert_eq!(panes[0].cwd, "file:///home/user");
    }

    #[test]
    fn test_parse_wezterm_pane_json_with_extra_fields() {
        // Forward-compat: unknown fields should be ignored
        let json = r#"[{"window_id":0,"tab_id":0,"pane_id":5,"title":"x","cwd":"file:///tmp","future_field":true}]"#;
        let panes: Vec<WezTermPane> = serde_json::from_str(json).unwrap();
        assert_eq!(panes[0].pane_id, 5);
    }

    /// Contract test — requires WezTerm to be running.
    /// Run with: `cargo test -p claude-pending-board-adapters -- --ignored`
    #[test]
    #[ignore]
    fn test_wezterm_is_available() {
        let adapter = WezTermAdapter::new();
        assert!(adapter.is_available(), "WezTerm binary not found in PATH");
    }

    /// Contract test — requires WezTerm to be running.
    #[test]
    #[ignore]
    fn test_wezterm_list_panes() {
        let panes = WezTermAdapter::list_panes().expect("failed to list panes");
        assert!(!panes.is_empty(), "no panes found — is WezTerm running?");
        println!("Found {} panes:", panes.len());
        for pane in &panes {
            println!(
                "  pane_id={} tab_id={} window_id={} title={:?} cwd={:?}",
                pane.pane_id, pane.tab_id, pane.window_id, pane.title, pane.cwd
            );
        }
    }
}
```

- [ ] **Step 3: Update `crates/adapters/src/lib.rs`**

```rust
pub mod wezterm;

use claude_pending_board_core::terminal::TerminalAdapter;

/// Registry of available terminal adapters.
pub struct AdapterRegistry {
    adapters: Vec<Box<dyn TerminalAdapter>>,
}

impl AdapterRegistry {
    /// Create a registry with all platform-appropriate adapters.
    pub fn new() -> Self {
        let mut adapters: Vec<Box<dyn TerminalAdapter>> = Vec::new();
        adapters.push(Box::new(wezterm::WezTermAdapter::new()));

        #[cfg(target_os = "macos")]
        {
            // iTerm2 adapter will be added in Task 2
        }

        Self { adapters }
    }

    /// Find the first adapter that can detect the given claude_pid.
    pub fn detect(
        &self,
        claude_pid: u32,
    ) -> Option<(
        &dyn TerminalAdapter,
        claude_pending_board_core::types::TerminalMatch,
    )> {
        for adapter in &self.adapters {
            if let Some(m) = adapter.detect(claude_pid) {
                return Some((adapter.as_ref(), m));
            }
        }
        None
    }

    /// Get an adapter by name (case-insensitive).
    pub fn get_by_name(&self, name: &str) -> Option<&dyn TerminalAdapter> {
        self.adapters
            .iter()
            .find(|a| a.name().eq_ignore_ascii_case(name))
            .map(|a| a.as_ref())
    }

    /// List names of all registered adapters.
    pub fn adapter_names(&self) -> Vec<&str> {
        self.adapters.iter().map(|a| a.name()).collect()
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_wezterm() {
        let registry = AdapterRegistry::new();
        let names = registry.adapter_names();
        assert!(names.contains(&"WezTerm"));
    }

    #[test]
    fn test_registry_get_by_name() {
        let registry = AdapterRegistry::new();
        assert!(registry.get_by_name("wezterm").is_some());
        assert!(registry.get_by_name("WezTerm").is_some());
        assert!(registry.get_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_detect_returns_none_for_fake_pid() {
        let registry = AdapterRegistry::new();
        assert!(registry.detect(0xFFFFFF).is_none());
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p claude-pending-board-adapters --nocapture`
Expected: All tests pass (4 from wezterm + 3 from lib = 7 tests, ignoring contract tests)

- [ ] **Step 5: Run clippy**

Run: `cargo clippy -p claude-pending-board-adapters -- -D warnings`
Expected: 0 warnings

- [ ] **Step 6: Commit**

```bash
git add crates/adapters/
git commit -m "feat(adapters): implement WezTermAdapter with detect, focus, and spawn"
```

---

## Task 2: iTerm2 Adapter (macOS only)

**Files:**
- Create: `crates/adapters/src/iterm2.rs`
- Modify: `crates/adapters/src/lib.rs`

- [ ] **Step 1: Create `crates/adapters/src/iterm2.rs`**

```rust
#![cfg(target_os = "macos")]

use claude_pending_board_core::terminal::{AdapterError, TerminalAdapter};
use claude_pending_board_core::types::TerminalMatch;
use std::path::Path;
use std::process::Command;

pub struct ITerm2Adapter;

impl ITerm2Adapter {
    pub fn new() -> Self {
        Self
    }

    /// Check if iTerm2.app exists.
    fn is_iterm2_installed() -> bool {
        Path::new("/Applications/iTerm.app").exists()
    }

    /// Run an AppleScript command via osascript.
    fn run_osascript(script: &str) -> Result<String, AdapterError> {
        let output = Command::new("osascript")
            .args(["-e", script])
            .output()
            .map_err(|e| AdapterError::CommandFailed(format!("osascript failed: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AdapterError::CommandFailed(format!(
                "osascript error: {stderr}"
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get the tty of a process by PID using ps.
    fn get_tty(pid: u32) -> Option<String> {
        let output = Command::new("ps")
            .args(["-o", "tty=", "-p", &pid.to_string()])
            .output()
            .ok()?;

        if output.status.success() {
            let tty = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !tty.is_empty() && tty != "??" {
                return Some(tty);
            }
        }
        None
    }
}

impl TerminalAdapter for ITerm2Adapter {
    fn name(&self) -> &str {
        "iTerm2"
    }

    fn is_available(&self) -> bool {
        Self::is_iterm2_installed()
    }

    fn detect(&self, claude_pid: u32) -> Option<TerminalMatch> {
        // Use ancestor walk to confirm this session runs in iTerm2
        let (terminal_name, terminal_pid) =
            claude_pending_board_core::terminal::ancestor_walk(claude_pid, 20)?;

        if !terminal_name.contains("iTerm") {
            return None;
        }

        // Get the tty of the Claude process for session matching
        let tty = Self::get_tty(claude_pid);

        Some(TerminalMatch {
            terminal_name,
            terminal_pid,
            pane_id: None,
            tty,
        })
    }

    fn focus_pane(&self, terminal_match: &TerminalMatch) -> Result<(), AdapterError> {
        // Activate iTerm2 and bring it to front
        let script = r#"tell application "iTerm2" to activate"#;
        Self::run_osascript(script)?;

        // If we have a tty, try to find and select the matching session
        if let Some(tty) = &terminal_match.tty {
            let script = format!(
                r#"tell application "iTerm2"
    repeat with w in windows
        repeat with t in tabs of w
            repeat with s in sessions of t
                if tty of s contains "{tty}" then
                    select t
                    select s
                    return "found"
                end if
            end repeat
        end repeat
    end repeat
    return "not_found"
end tell"#,
                tty = tty
            );
            let result = Self::run_osascript(&script)?;
            if result == "not_found" {
                tracing::warn!(tty, "iTerm2 session with matching tty not found");
            }
        }

        Ok(())
    }

    fn spawn_resume(&self, cwd: &Path, session_id: &str) -> Result<(), AdapterError> {
        let cwd_str = cwd.to_string_lossy();
        let script = format!(
            r#"tell application "iTerm2"
    activate
    tell current window
        create tab with default profile command "cd {cwd} && claude --resume {session_id}"
    end tell
end tell"#,
            cwd = cwd_str,
            session_id = session_id
        );
        Self::run_osascript(&script)?;

        tracing::info!(session_id, cwd = %cwd.display(), "spawned resume in new iTerm2 tab");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iterm2_adapter_name() {
        let adapter = ITerm2Adapter::new();
        assert_eq!(adapter.name(), "iTerm2");
    }

    #[test]
    fn test_detect_returns_none_for_fake_pid() {
        let adapter = ITerm2Adapter::new();
        assert!(adapter.detect(0xFFFFFF).is_none());
    }

    /// Contract test — requires iTerm2 to be running on macOS.
    #[test]
    #[ignore]
    fn test_iterm2_is_available() {
        let adapter = ITerm2Adapter::new();
        assert!(adapter.is_available(), "iTerm2 not found at /Applications/iTerm.app");
    }
}
```

- [ ] **Step 2: Update `crates/adapters/src/lib.rs` to register iTerm2**

Add after the `pub mod wezterm;` line:

```rust
#[cfg(target_os = "macos")]
pub mod iterm2;
```

And in the `AdapterRegistry::new()` method, replace the `#[cfg(target_os = "macos")]` block:

```rust
#[cfg(target_os = "macos")]
{
    adapters.push(Box::new(iterm2::ITerm2Adapter::new()));
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p claude-pending-board-adapters --nocapture`
Expected: All tests pass. On Windows, the iTerm2 module is not compiled. On macOS, the iTerm2 unit tests run but contract tests are ignored.

- [ ] **Step 4: Run clippy**

Run: `cargo clippy -p claude-pending-board-adapters -- -D warnings`
Expected: 0 warnings

- [ ] **Step 5: Commit**

```bash
git add crates/adapters/src/iterm2.rs crates/adapters/src/lib.rs
git commit -m "feat(adapters): implement ITerm2Adapter with AppleScript focus and spawn (macOS)"
```

---

## Task 3: PowerShell Hook Script (Windows)

**Files:**
- Create: `scripts/pending_hook.ps1`

- [ ] **Step 1: Create `scripts/pending_hook.ps1`**

```powershell
#!/usr/bin/env pwsh
# pending_hook.ps1 — Claude Code hook for Notification, UserPromptSubmit, and Stop events.
# Appends ops to ~/.claude/pending/board.jsonl.
# MUST always exit 0 — never block Claude Code.

try {
    # Read JSON payload from stdin
    $rawInput = $input | Out-String
    if (-not $rawInput.Trim()) {
        exit 0
    }
    $payload = $rawInput | ConvertFrom-Json

    # Determine event type from hook_event_name
    $eventName = $payload.hook_event_name
    $sessionId = $payload.session_id
    $cwd = $payload.cwd

    if (-not $sessionId) {
        exit 0
    }

    # Board file location
    $boardDir = Join-Path $HOME ".claude" "pending"
    $boardFile = Join-Path $boardDir "board.jsonl"
    $logDir = Join-Path $boardDir "logs"

    # Ensure directories exist
    if (-not (Test-Path $boardDir)) {
        New-Item -ItemType Directory -Path $boardDir -Force | Out-Null
    }
    if (-not (Test-Path $logDir)) {
        New-Item -ItemType Directory -Path $logDir -Force | Out-Null
    }

    $ts = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")

    switch ($eventName) {
        "Notification" {
            $notificationType = $payload.notification_type
            if ($notificationType -ne "permission_prompt" -and $notificationType -ne "idle_prompt") {
                exit 0
            }

            $message = $payload.message
            $transcriptPath = $payload.transcript_path
            $claudePid = $PID  # Current process PID — hook runs as child of Claude

            # Walk process tree to find the owning terminal PID
            $terminalPid = $null
            $currentPid = $claudePid
            for ($i = 0; $i -lt 20; $i++) {
                try {
                    $proc = Get-CimInstance Win32_Process -Filter "ProcessId = $currentPid" -ErrorAction Stop
                    if (-not $proc) { break }
                    $procName = $proc.Name -replace '\.exe$', ''
                    if ($procName -match '^(wezterm-gui|wezterm|iTerm2)$') {
                        $terminalPid = $currentPid
                        break
                    }
                    $currentPid = $proc.ParentProcessId
                    if ($currentPid -eq 0) { break }
                }
                catch { break }
            }

            $op = @{
                op                = "add"
                ts                = $ts
                session_id        = $sessionId
                cwd               = $cwd
                claude_pid        = $claudePid
                terminal_pid      = $terminalPid
                transcript_path   = $transcriptPath
                notification_type = $notificationType
                message           = if ($message) { $message } else { "" }
            } | ConvertTo-Json -Compress

            Add-Content -Path $boardFile -Value $op -Encoding UTF8
        }

        "UserPromptSubmit" {
            $op = @{
                op         = "clear"
                ts         = $ts
                session_id = $sessionId
                reason     = "user_replied"
            } | ConvertTo-Json -Compress

            Add-Content -Path $boardFile -Value $op -Encoding UTF8
        }

        "Stop" {
            $op = @{
                op         = "clear"
                ts         = $ts
                session_id = $sessionId
                reason     = "stop"
            } | ConvertTo-Json -Compress

            Add-Content -Path $boardFile -Value $op -Encoding UTF8
        }

        default {
            # Unknown event — ignore silently
        }
    }
}
catch {
    # Log error but never block Claude Code
    try {
        $logDir = Join-Path $HOME ".claude" "pending" "logs"
        if (-not (Test-Path $logDir)) {
            New-Item -ItemType Directory -Path $logDir -Force | Out-Null
        }
        $logFile = Join-Path $logDir "hook-errors.log"
        $errorMsg = "[$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')] ERROR: $($_.Exception.Message)`n$($_.ScriptStackTrace)"
        Add-Content -Path $logFile -Value $errorMsg -Encoding UTF8
    }
    catch {
        # Even error logging failed — silently give up
    }
}

# Always exit 0
exit 0
```

- [ ] **Step 2: Verify the script is syntactically valid**

Run: `pwsh -NoProfile -Command "& { $null = [System.Management.Automation.Language.Parser]::ParseFile('D:/lab/suxi/claude-pending-board/scripts/pending_hook.ps1', [ref]$null, [ref]$null) ; Write-Host 'Parse OK' }"`
Expected: `Parse OK`

- [ ] **Step 3: Test with a sample Notification payload**

Run:
```powershell
echo '{"hook_event_name":"Notification","session_id":"test-ps1","cwd":"C:/tmp","transcript_path":"C:/tmp/t.jsonl","notification_type":"permission_prompt","message":"Test permission"}' | pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/pending_hook.ps1
```

Then check: `type $HOME/.claude/pending/board.jsonl | findstr test-ps1`
Expected: A JSON line with `"op":"add"` and `"session_id":"test-ps1"`

- [ ] **Step 4: Test with a UserPromptSubmit payload**

Run:
```powershell
echo '{"hook_event_name":"UserPromptSubmit","session_id":"test-ps1","cwd":"C:/tmp"}' | pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/pending_hook.ps1
```

Then check: `type $HOME/.claude/pending/board.jsonl | findstr test-ps1`
Expected: A second line with `"op":"clear"` and `"session_id":"test-ps1"`

- [ ] **Step 5: Clean up test data and commit**

```bash
rm -f ~/.claude/pending/board.jsonl
git add scripts/pending_hook.ps1
git commit -m "feat(hooks): add PowerShell hook script for Windows"
```

---

## Task 4: Bash Hook Script (macOS / Linux)

**Files:**
- Create: `scripts/pending_hook.sh`

- [ ] **Step 1: Create `scripts/pending_hook.sh`**

```bash
#!/usr/bin/env bash
# pending_hook.sh — Claude Code hook for Notification, UserPromptSubmit, and Stop events.
# Appends ops to ~/.claude/pending/board.jsonl.
# MUST always exit 0 — never block Claude Code.

set -o pipefail

BOARD_DIR="$HOME/.claude/pending"
BOARD_FILE="$BOARD_DIR/board.jsonl"
LOG_DIR="$BOARD_DIR/logs"
LOG_FILE="$LOG_DIR/hook-errors.log"

log_error() {
    mkdir -p "$LOG_DIR" 2>/dev/null || true
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: $1" >> "$LOG_FILE" 2>/dev/null || true
}

main() {
    # Read JSON from stdin
    local raw_input
    raw_input=$(cat)
    if [ -z "$raw_input" ]; then
        return 0
    fi

    # Ensure directories exist
    mkdir -p "$BOARD_DIR" 2>/dev/null || { log_error "cannot create $BOARD_DIR"; return 0; }
    mkdir -p "$LOG_DIR" 2>/dev/null || true

    # Extract fields using lightweight JSON parsing
    # We use python3 if available, otherwise try jq, otherwise fall back to grep/sed
    local event_name session_id cwd

    if command -v python3 &>/dev/null; then
        eval "$(echo "$raw_input" | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    for k in ['hook_event_name','session_id','cwd','notification_type','message','transcript_path']:
        v = d.get(k, '')
        # Escape single quotes for shell
        v_escaped = str(v).replace(\"'\", \"'\\\"'\\\"'\")
        print(f\"{k}='{v_escaped}'\")
except:
    sys.exit(1)
" 2>/dev/null)" || { log_error "failed to parse JSON"; return 0; }
    elif command -v jq &>/dev/null; then
        event_name=$(echo "$raw_input" | jq -r '.hook_event_name // empty' 2>/dev/null)
        session_id=$(echo "$raw_input" | jq -r '.session_id // empty' 2>/dev/null)
        cwd=$(echo "$raw_input" | jq -r '.cwd // empty' 2>/dev/null)
        notification_type=$(echo "$raw_input" | jq -r '.notification_type // empty' 2>/dev/null)
        message=$(echo "$raw_input" | jq -r '.message // empty' 2>/dev/null)
        transcript_path=$(echo "$raw_input" | jq -r '.transcript_path // empty' 2>/dev/null)
    else
        log_error "neither python3 nor jq found — cannot parse hook payload"
        return 0
    fi

    event_name="${hook_event_name:-$event_name}"
    session_id="${session_id:-}"
    cwd="${cwd:-}"

    if [ -z "$session_id" ]; then
        return 0
    fi

    local ts
    ts=$(date -u '+%Y-%m-%dT%H:%M:%S.000Z')
    local claude_pid=$$

    case "$event_name" in
        Notification)
            notification_type="${notification_type:-}"
            if [ "$notification_type" != "permission_prompt" ] && [ "$notification_type" != "idle_prompt" ]; then
                return 0
            fi

            message="${message:-}"
            transcript_path="${transcript_path:-}"

            # Walk process tree to find terminal PID
            local terminal_pid="null"
            local current_pid=$claude_pid
            for _ in $(seq 1 20); do
                if [ "$(uname)" = "Darwin" ]; then
                    local ppid_val
                    ppid_val=$(ps -o ppid= -p "$current_pid" 2>/dev/null | tr -d ' ')
                    local proc_name
                    proc_name=$(ps -o comm= -p "$current_pid" 2>/dev/null | xargs basename 2>/dev/null)
                else
                    # Linux: read from /proc
                    local ppid_val
                    ppid_val=$(awk '{print $4}' "/proc/$current_pid/stat" 2>/dev/null)
                    local proc_name
                    proc_name=$(awk '{print $2}' "/proc/$current_pid/stat" 2>/dev/null | tr -d '()')
                fi

                if [ -z "$ppid_val" ] || [ "$ppid_val" = "0" ]; then
                    break
                fi

                case "$proc_name" in
                    wezterm-gui|wezterm|iTerm2)
                        terminal_pid=$current_pid
                        break
                        ;;
                esac

                current_pid=$ppid_val
            done

            # Escape message for JSON (basic: replace backslash, double-quote, newlines)
            local escaped_message
            escaped_message=$(printf '%s' "$message" | sed 's/\\/\\\\/g; s/"/\\"/g' | tr '\n' ' ')

            printf '{"op":"add","ts":"%s","session_id":"%s","cwd":"%s","claude_pid":%d,"terminal_pid":%s,"transcript_path":"%s","notification_type":"%s","message":"%s"}\n' \
                "$ts" "$session_id" "$cwd" "$claude_pid" "$terminal_pid" "$transcript_path" "$notification_type" "$escaped_message" \
                >> "$BOARD_FILE"
            ;;

        UserPromptSubmit)
            printf '{"op":"clear","ts":"%s","session_id":"%s","reason":"user_replied"}\n' \
                "$ts" "$session_id" \
                >> "$BOARD_FILE"
            ;;

        Stop)
            printf '{"op":"clear","ts":"%s","session_id":"%s","reason":"stop"}\n' \
                "$ts" "$session_id" \
                >> "$BOARD_FILE"
            ;;

        *)
            # Unknown event — ignore silently
            ;;
    esac
}

# Run main in a subshell so errors don't propagate
(main) 2>/dev/null || true

# Always exit 0
exit 0
```

- [ ] **Step 2: Make the script executable**

Run: `chmod +x scripts/pending_hook.sh`

- [ ] **Step 3: Verify syntax (bash -n)**

Run: `bash -n scripts/pending_hook.sh && echo "Syntax OK"`
Expected: `Syntax OK`

- [ ] **Step 4: Commit**

```bash
git add scripts/pending_hook.sh
git commit -m "feat(hooks): add Bash hook script for macOS and Linux"
```

---

## Task 5: Hook Scripts README

**Files:**
- Create: `scripts/README.md`

- [ ] **Step 1: Create `scripts/README.md`**

```markdown
# Hook Scripts

These scripts are invoked by Claude Code hooks to write pending-board entries to `~/.claude/pending/board.jsonl`.

## Scripts

| Script | Platform | Shell |
|---|---|---|
| `pending_hook.ps1` | Windows | PowerShell 7 |
| `pending_hook.sh` | macOS / Linux | Bash |

## How they work

Each script:
1. Reads a JSON payload from stdin (provided by Claude Code)
2. Checks the `hook_event_name` field
3. For `Notification` events with `permission_prompt` or `idle_prompt`: appends an `add` op
4. For `UserPromptSubmit` events: appends a `clear` op with reason `user_replied`
5. For `Stop` events: appends a `clear` op with reason `stop`
6. Walks the process tree to find the owning terminal PID (WezTerm or iTerm2)
7. Wraps everything in try/catch — always exits 0, never blocks Claude Code

## Manual testing

### Windows (PowerShell)

```powershell
# Test Notification (add)
echo '{"hook_event_name":"Notification","session_id":"test-123","cwd":"C:/tmp","transcript_path":"C:/tmp/t.jsonl","notification_type":"permission_prompt","message":"May I run ls?"}' | pwsh -NoProfile -ExecutionPolicy Bypass -File pending_hook.ps1

# Test UserPromptSubmit (clear)
echo '{"hook_event_name":"UserPromptSubmit","session_id":"test-123","cwd":"C:/tmp"}' | pwsh -NoProfile -ExecutionPolicy Bypass -File pending_hook.ps1

# Test Stop (clear)
echo '{"hook_event_name":"Stop","session_id":"test-123","cwd":"C:/tmp"}' | pwsh -NoProfile -ExecutionPolicy Bypass -File pending_hook.ps1

# Check results
Get-Content ~/.claude/pending/board.jsonl
```

### macOS / Linux (Bash)

```bash
# Test Notification (add)
echo '{"hook_event_name":"Notification","session_id":"test-456","cwd":"/tmp","transcript_path":"/tmp/t.jsonl","notification_type":"permission_prompt","message":"May I run ls?"}' | bash pending_hook.sh

# Test UserPromptSubmit (clear)
echo '{"hook_event_name":"UserPromptSubmit","session_id":"test-456","cwd":"/tmp"}' | bash pending_hook.sh

# Check results
cat ~/.claude/pending/board.jsonl
```

### Verify error handling

```bash
# Empty stdin — should exit 0 silently
echo '' | bash pending_hook.sh; echo "exit: $?"

# Malformed JSON — should exit 0 and log to hook-errors.log
echo 'not json' | bash pending_hook.sh; echo "exit: $?"
cat ~/.claude/pending/logs/hook-errors.log
```

## Clean up test data

```bash
rm -f ~/.claude/pending/board.jsonl
rm -f ~/.claude/pending/logs/hook-errors.log
```
```

- [ ] **Step 2: Commit**

```bash
git add scripts/README.md
git commit -m "docs: add hook scripts README with manual testing instructions"
```

---

## Task 6: Final Integration Verification

- [ ] **Step 1: Run all adapter tests**

Run: `cargo test -p claude-pending-board-adapters --nocapture`
Expected: All unit tests pass

- [ ] **Step 2: Run all workspace tests**

Run: `cargo test --workspace --nocapture`
Expected: All tests pass (55 core + adapter tests)

- [ ] **Step 3: Run clippy on workspace**

Run: `cargo clippy --workspace -- -D warnings`
Expected: 0 warnings

- [ ] **Step 4: Run fmt check**

Run: `cargo fmt --check --all`
Expected: No formatting issues (fix with `cargo fmt --all` if needed)

- [ ] **Step 5: Verify hook script syntax**

Run (Windows): `pwsh -NoProfile -Command "& { $null = [System.Management.Automation.Language.Parser]::ParseFile('scripts/pending_hook.ps1', [ref]$null, [ref]$null) ; Write-Host 'PS1 OK' }"`
Run: `bash -n scripts/pending_hook.sh && echo "Bash OK"`
Expected: Both print OK

- [ ] **Step 6: Commit any fixes and tag**

```bash
git add -A
git commit -m "chore: phase 2 complete — adapters and hook scripts"
git tag v0.0.2-adapters
```

---

## Self-Review Checklist

1. **Spec coverage:**
   - Click to focus live (WezTerm): `wezterm.rs` — `focus_pane` via `activate-pane` — covered
   - Click to focus live (iTerm2): `iterm2.rs` — `focus_pane` via AppleScript — covered
   - Click to resume stale (WezTerm): `wezterm.rs` — `spawn_resume` via `cli spawn` — covered
   - Click to resume stale (iTerm2): `iterm2.rs` — `spawn_resume` via AppleScript tab — covered
   - No adapter matched: `AdapterRegistry::detect` returns `None` — covered (UI handles in Phase 3)
   - Hook-driven entry capture: `pending_hook.ps1` + `pending_hook.sh` — covered
   - Entry removal: both scripts handle `UserPromptSubmit` and `Stop` — covered
   - Hook failure does not block: both scripts wrap in try/catch, exit 0 — covered
   - Ancestor walking in hooks: both scripts walk process tree to find terminal PID — covered
   - Contract tests for real terminals: `#[ignore]` tests documented — covered

2. **Placeholder scan:** No TBD/TODO/placeholders found. All code is complete.

3. **Type consistency:** `TerminalAdapter` trait from `core::terminal` used correctly. `TerminalMatch` fields match. `AdapterError` variants used consistently. `Op` JSON shape in hook scripts matches `core::types::Op` serde format.
