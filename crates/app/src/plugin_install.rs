//! Detect whether Claude Code's pending-board plugin is installed and, when
//! asked, install it by shelling out to the `claude` CLI.
//!
//! Architecture note: the tray app installer only drops the binary — it does
//! not touch `~/.claude/settings.json`. The hooks are owned by the Claude
//! Code plugin system, so we drive install via `claude plugin marketplace
//! add ...` + `claude plugin install ...`. This runs as the current user (not
//! the MSI's SYSTEM context) because we shell out from the already-running
//! tray process.

use std::process::{Command, Stdio};

// The CLI's `plugin marketplace add` accepts owner/repo, a full URL, or a
// local path — NOT the `github:owner/repo` short-form the Claude Code
// slash command accepts. See `claude plugin marketplace add --help`.
const MARKETPLACE: &str = "sadwx/claude-pending-board";
const PLUGIN_REF: &str = "claude-pending-board@claude-pending-board";
const PLUGIN_NAME: &str = "claude-pending-board";

#[derive(Debug, serde::Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HookStatus {
    /// Plugin is installed and enabled.
    Installed,
    /// Plugin is not installed (but `claude` CLI is present).
    NotInstalled,
    /// `claude` CLI is not in PATH — user needs to install Claude Code first.
    CliMissing,
}

pub fn detect() -> HookStatus {
    let Some(output) = run_claude(&["plugin", "list"]) else {
        return HookStatus::CliMissing;
    };
    if !output.status.success() {
        // `claude plugin list` failing usually means auth / init issues, not
        // a missing CLI. Treat as not installed so the user can retry.
        return HookStatus::NotInstalled;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.contains(PLUGIN_NAME) {
        HookStatus::Installed
    } else {
        HookStatus::NotInstalled
    }
}

pub fn install() -> Result<(), String> {
    let Some(add_output) = run_claude(&["plugin", "marketplace", "add", MARKETPLACE]) else {
        return Err(cli_missing_msg());
    };
    if !add_output.status.success() {
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        // Idempotent: "already exists" from a prior install is fine.
        if !stderr.to_ascii_lowercase().contains("already") {
            return Err(format!(
                "claude plugin marketplace add failed: {}",
                stderr.trim()
            ));
        }
    }

    let Some(install_output) = run_claude(&["plugin", "install", PLUGIN_REF]) else {
        return Err(cli_missing_msg());
    };
    if !install_output.status.success() {
        let stderr = String::from_utf8_lossy(&install_output.stderr);
        return Err(format!("claude plugin install failed: {}", stderr.trim()));
    }

    Ok(())
}

fn run_claude(args: &[&str]) -> Option<std::process::Output> {
    match Command::new("claude")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(o) => Some(o),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            tracing::warn!(error = %e, args = ?args, "`claude` invocation failed");
            None
        }
    }
}

fn cli_missing_msg() -> String {
    "`claude` CLI not found in PATH. Install Claude Code first, then try again.".to_string()
}
