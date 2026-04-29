//! Regression guard: hook scripts MUST live only in `plugin/hooks/`. The
//! single source of truth is what the plugin marketplace ships; copying
//! them to `scripts/` reintroduces the drift problem we deleted them to
//! fix.
//!
//! If this test fails, delete the `scripts/pending_hook.*` files and
//! update any docs that point at them to use `plugin/hooks/` instead.

use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR for this crate is `<workspace>/crates/core`.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/<crate> parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn scripts_dir_must_not_duplicate_plugin_hooks() {
    let root = workspace_root();
    let plugin_sh = root.join("plugin").join("hooks").join("pending_hook.sh");
    let plugin_ps1 = root.join("plugin").join("hooks").join("pending_hook.ps1");
    assert!(
        plugin_sh.is_file(),
        "expected source-of-truth hook missing: {}",
        plugin_sh.display()
    );
    assert!(
        plugin_ps1.is_file(),
        "expected source-of-truth hook missing: {}",
        plugin_ps1.display()
    );

    let dup_sh = root.join("scripts").join("pending_hook.sh");
    let dup_ps1 = root.join("scripts").join("pending_hook.ps1");
    assert!(
        !dup_sh.exists(),
        "scripts/pending_hook.sh must not exist — single source of truth is plugin/hooks/pending_hook.sh"
    );
    assert!(
        !dup_ps1.exists(),
        "scripts/pending_hook.ps1 must not exist — single source of truth is plugin/hooks/pending_hook.ps1"
    );
}
