# scripts/

Smoke-test scripts and other dev utilities for working on the tray app.

## Scripts

| Script | Purpose |
|---|---|
| `smoke-test.ps1` | Interactive end-to-end smoke test for humans (Windows). |
| `smoke-test-auto.ps1` | Non-interactive smoke test (~40 s) suitable for CI / quick local verification. |
| `smoke-test.sh` | macOS / Linux equivalent of `smoke-test.ps1`. |

## Hook scripts

The Claude Code hook scripts (`pending_hook.sh` and `pending_hook.ps1`) live in [`plugin/hooks/`](../plugin/hooks/) — that directory is the single source of truth and what the plugin marketplace ships. There is no copy in `scripts/`; if you need to invoke a hook directly while iterating, point at the plugin path:

```powershell
# Windows
'{"hook_event_name":"Notification","session_id":"...","cwd":"...","transcript_path":"...","notification_type":"permission_prompt","message":"Test"}' | pwsh -File plugin/hooks/pending_hook.ps1
```

```bash
# macOS / WSL
echo '{"hook_event_name":"Notification","session_id":"...","cwd":"...","transcript_path":"...","notification_type":"permission_prompt","message":"Test"}' | bash plugin/hooks/pending_hook.sh
```

After running, check `~/.claude/pending/board.jsonl` for the appended op.
