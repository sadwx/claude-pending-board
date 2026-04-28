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

            # WezTerm injects $env:WEZTERM_PANE into every shell it spawns —
            # capture it so click-to-focus can call `wezterm cli activate-pane`
            # directly instead of walking the process tree (which picks the
            # wrong pane when the user has multiple wezterm tabs).
            $wezTermPaneId = $env:WEZTERM_PANE

            $op = [ordered]@{
                op                = "add"
                ts                = $ts
                session_id        = $sessionId
                cwd               = $cwd
                claude_pid        = $claudePid
                terminal_pid      = $terminalPid
                transcript_path   = $transcriptPath
                notification_type = $notificationType
                message           = if ($message) { $message } else { "" }
            }
            if ($wezTermPaneId) {
                $op.wezterm_pane_id = $wezTermPaneId
            }

            Add-Content -Path $boardFile -Value ($op | ConvertTo-Json -Compress) -Encoding UTF8
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

        "SessionEnd" {
            # Fires on `/clear`, `/compact`, normal exit, or any other path
            # that terminates the session. Treat all of these as "this entry
            # is no longer waiting for me" and drop it from the HUD. Stop
            # already covers the post-reply path, but it does NOT fire on
            # `/clear` — SessionEnd is the only signal there.
            $op = @{
                op         = "clear"
                ts         = $ts
                session_id = $sessionId
                reason     = "session_ended"
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
