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
                local ppid_val
                ppid_val=$(ps -o ppid= -p "$current_pid" 2>/dev/null | tr -d ' ')
                local proc_name
                proc_name=$(ps -o comm= -p "$current_pid" 2>/dev/null | xargs basename 2>/dev/null)

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

            # When running inside WSL, tag the entry with the distro name so
            # the Windows-side reaper / WezTerm adapter can route correctly.
            # Field is omitted entirely on macOS (and any non-WSL Linux).
            local wsl_distro_field=""
            if [ -n "${WSL_DISTRO_NAME:-}" ]; then
                wsl_distro_field=$(printf ',"wsl_distro":"%s"' "$WSL_DISTRO_NAME")
            fi

            # WezTerm injects $WEZTERM_PANE into every shell it spawns. Capture
            # it so click-to-focus can address the exact pane via
            # `wezterm cli activate-pane --pane-id <id>` instead of guessing
            # via the process tree (which fails for WSL — claude_pid lives in
            # WSL's pid namespace — and picks the wrong pane on Windows when
            # the user has multiple wezterm tabs).
            #
            # WSL note: requires `WSLENV=WEZTERM_PANE/u` so the env var
            # crosses the Win→WSL boundary (see INSTALL.md).
            local wezterm_pane_field=""
            if [ -n "${WEZTERM_PANE:-}" ]; then
                wezterm_pane_field=$(printf ',"wezterm_pane_id":"%s"' "$WEZTERM_PANE")
            fi

            printf '{"op":"add","ts":"%s","session_id":"%s","cwd":"%s","claude_pid":%d,"terminal_pid":%s,"transcript_path":"%s","notification_type":"%s","message":"%s"%s%s}\n' \
                "$ts" "$session_id" "$cwd" "$claude_pid" "$terminal_pid" "$transcript_path" "$notification_type" "$escaped_message" "$wsl_distro_field" "$wezterm_pane_field" \
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
