# Tasks — WSL support

Implementation is split across three PRs. PR-A and PR-C are the critical path; PR-B is a manifest tweak that can land any time after PR-A.

## PR-A · Reaper short-circuits WSL entries

- [ ] Add `pub wsl_distro: Option<String>` to `Entry` in `crates/core/src/types.rs`.
- [ ] Add the field to the `Op::Add` variant with `#[serde(skip_serializing_if = "Option::is_none")]` so absent → omitted in JSON.
- [ ] Thread the field through `StateStore::apply` in `crates/core/src/board/store.rs` and through the parser in `crates/core/src/board/parser.rs`.
- [ ] In `crates/core/src/reaper.rs::check_liveness`, return `LivenessResult::Alive` immediately when `entry.wsl_distro.is_some()`.
- [ ] Update `crates/core/src/board/compaction.rs` to round-trip the field on rewrite.
- [ ] Tests: unit test in `reaper.rs` that an entry with `wsl_distro = Some("Ubuntu-24.04")` and a dead `claude_pid` resolves to `Alive`. Round-trip parser test with the new field present and absent.

## PR-C · WezTerm adapter — distro-aware spawn + resume

- [ ] Update `scripts/pending_hook.sh` and `plugin/hooks/pending_hook.sh` to emit `"wsl_distro": "<name>"` when `$WSL_DISTRO_NAME` is set; absent otherwise.
- [ ] In `crates/adapters/src/wezterm.rs::spawn_resume`, branch on whether the entry's `cwd` came with a `wsl_distro`.
- [ ] Add a helper `wsl_cwd_to_unc(distro: &str, linux_cwd: &Path) -> String` that produces `\\wsl$\<distro>\<rest>`. Pure string transform, no I/O.
- [ ] Build the resume command as `wsl.exe -d <distro> -e claude --resume <session_id>` and pass `--cwd <unc>` to `wezterm cli spawn`.
- [ ] Threading note: `Adapter::spawn_resume` currently takes `(cwd, session_id)`. Widen to `(cwd, session_id, wsl_distro: Option<&str>)` to keep the diff small. (Passing the whole `Entry` is a future option but premature for this PR.)
- [ ] Tests: `wsl_cwd_to_unc("Ubuntu-24.04", Path::new("/home/simon/project")) → "\\\\wsl$\\Ubuntu-24.04\\home\\simon\\project"` etc.
- [ ] Manual smoke on Simon's WSL: trigger a permission prompt in WSL Claude, verify HUD entry stays live, click → new WezTerm tab opens with WSL cwd and resumes the session.

## PR-B · Plugin manifest re-introduces Linux platform

- [ ] In `plugin/.claude-plugin/plugin.json`, mirror each existing `platform: "darwin"` bash entry with a `platform: "linux"` entry pointing at the same script.
- [ ] In `scripts/pending_hook.sh` (the source of truth), confirm no Linux-specific assumptions broke during the `af26a1e` revert window — the `ps -o ppid=` branch is fine for both macOS and Linux.
- [ ] Update `INSTALL.md` to mention WSL works (with caveats — entry takes 2–5 s to appear; click-to-focus needs WezTerm on the Windows side).

## Validation gate (before tagging v0.2)

- [ ] PR-A merged: WSL PoC entries on Simon's machine no longer go stale within seconds. Wait at least one reaper sweep (30 s) after firing a WSL hook and confirm the entry is still `Live`.
- [ ] PR-C merged: clicking a WSL entry opens a WezTerm tab inside the right distro and runs `claude --resume`; the prompt actually returns to the user.
- [ ] PR-B merged: in WSL, `claude plugin install claude-pending-board@claude-pending-board` registers all three hooks without manual settings.json editing. Simon reverts his PoC settings.json (backup is at `~/.claude/settings.json.pre-pending-board-poc`).
- [ ] Documentation: `INSTALL.md` has a "WSL" section with the caveats listed above.

## Deferred to a later change

- WSL liveness via `wsl.exe -d <distro> -e ps -p <pid>` instead of skipping the check entirely. Worth doing once we have a way to amortize the per-call cost (sweep batching, or a long-lived `wsl.exe -e bash` shell maintained by the adapter).
- Native Linux desktop support. Independent decision; not implied by this change.
