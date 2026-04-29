# Design — WSL support for the pending board

## Detection: how do we know an entry came from WSL?

Two options were considered:

1. **Heuristic on `cwd`** — anything starting with `/` and not on macOS is "probably WSL". Cheap, no schema change.
2. **Explicit `wsl_distro` field on the entry** — populated only when the hook ran inside WSL.

We chose option 2. The heuristic is fragile (macOS users have `/` paths too; future native-Linux re-introduction would need to disambiguate; the adapter needs the distro name *anyway* to translate cwd into a `\\wsl$\<distro>\…` UNC path). One explicit field is cleaner than three derived ones.

The bash hook already runs `printf` to construct the JSON line; adding one more conditional field is a few extra lines:

```bash
if [ -n "${WSL_DISTRO_NAME:-}" ]; then
  wsl_distro_field=",\"wsl_distro\":\"$WSL_DISTRO_NAME\""
else
  wsl_distro_field=""
fi
```

`Entry` and `Op::Add` in `crates/core/src/types.rs` gain an `Option<String>` field. Serde flattens `None` to absence with `#[serde(skip_serializing_if = "Option::is_none")]` so existing macOS / Windows board files stay byte-identical.

## Reaper

Current behavior at `crates/core/src/reaper.rs`:

```rust
if !proc_table.is_alive(entry.claude_pid) {
    return LivenessResult::Dead;
}
match session_files.read_session_id(entry.claude_pid) { … }
```

`proc_table.is_alive` walks the **Windows** process table on Windows, **macOS** processes on macOS — never the WSL side. A WSL pid never matches.

**Fix (PR-A):** in `check_liveness`, short-circuit when the entry has `Some(wsl_distro)`:

```rust
if entry.wsl_distro.is_some() {
    return LivenessResult::Alive;
}
```

This is "trust the hook stream". WSL Claude is going to send a `Stop` or `UserPromptSubmit` op when the session ends, and the entry will clear via that path. If the user kills the WSL terminal without exiting cleanly, the entry will linger until the periodic stale cleanup loop (1 hour TTL, see PR #10) drops it. That's a worse fate than what native entries get, but it's correct, conservative, and avoids the `wsl.exe ps` cross-boundary call which would slow the sweep loop significantly.

A future iteration can add a real WSL liveness probe; for v0.2 the conservative skip is the right shape.

## Plugin manifest

`plugin/.claude-plugin/plugin.json` currently gates the bash hook to `platform: "darwin"`. The fix is mechanical — re-add `platform: "linux"` mirroring each existing macOS entry. After this lands, `claude plugin install` from inside WSL registers the hooks the same way it does on macOS today, no hand-rolled `~/.claude/settings.json` editing required.

We deliberately do **not** restore the Linux entries to `release.yml` (no `.deb` / `.AppImage` build) or to the CI test matrix. Native Linux desktop support remains out of scope; only the plugin half of "Linux support" comes back, and only because WSL needs it.

## WezTerm adapter — cwd translation

When the WezTerm adapter receives a focus / spawn request for an entry with `Some(wsl_distro)`, the existing `cwd` (a Linux path like `/home/user/project`) is unusable. WSL exposes Linux files to Windows under `\\wsl$\<distro>\…`, so:

```
/home/user/project      →    \\wsl$\Ubuntu-24.04\home\user\project
/var/log                 →    \\wsl$\Ubuntu-24.04\var\log
```

The translation is a string-prefix swap, no escaping, no path canonicalization. Implementation lives entirely in the adapter — `core` and the hook stay pure.

## WezTerm adapter — resume command

`claude --resume <session_id>` on Windows looks at `~/.claude/sessions/<pid>.json` files written by Windows-side Claude. WSL sessions are never registered there. To resume a WSL session, the resume command must run **inside** the same WSL distro that produced the entry:

```
wezterm cli spawn --cwd '\\wsl$\Ubuntu-24.04\home\user\project' \
  -- wsl.exe -d Ubuntu-24.04 -e claude --resume <session_id>
```

`wezterm cli spawn` runs in the Windows wezterm-mux. The `--` introduces the program to launch in the new tab; `wsl.exe -d <distro> -e <cmd>` runs `<cmd>` inside the named distro with stdio attached.

The existing live-entry focus path (which uses `wezterm cli list` + `activate-pane` against a `terminal_pid`) cannot be made to work for WSL-origin entries — `terminal_pid` from a WSL hook refers to a process inside WSL, not a Windows-side WezTerm pane. WSL live-entry clicks therefore fall through to the spawn path (now correct).

## Schema migration

The new `wsl_distro` field is purely additive. Old `Op::Add` lines (no field) deserialize with `wsl_distro: None`; new lines round-trip cleanly through the on-disk JSONL. Compaction rewrites lines using `serde_json::to_string`, so existing boards from v0.1.x stay readable and the new field appears only on entries written by an updated bash hook.

## Out-of-scope alternatives considered

- **Run the tray app inside WSL too.** Would need restoring Linux Tauri build, GTK / WebKit deps, and would still leave Windows users with two trays. Rejected.
- **Cross-boundary file events via `inotifywait` proxy.** Current 9P → ReadDirectoryChangesW path works (~2–5 s); not worth a dedicated proxy.
- **Distro discovery from the adapter side.** Could `wsl.exe -l` to enumerate distros and guess. The hook already has `$WSL_DISTRO_NAME` for free; threading it through the entry is simpler.
