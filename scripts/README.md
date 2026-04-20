# Hook Scripts

These scripts are invoked by Claude Code hooks to write pending-board entries to `~/.claude/pending/board.jsonl`.

## Scripts

| Script | Platform | Shell |
|---|---|---|
| `pending_hook.ps1` | Windows | PowerShell 7 |
| `pending_hook.sh` | macOS | Bash |

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

### macOS (Bash)

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
