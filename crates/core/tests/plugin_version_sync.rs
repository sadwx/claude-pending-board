//! Regression guard: `plugin/.claude-plugin/plugin.json`'s `version` field
//! must agree with the workspace `Cargo.toml` `[workspace.package]`
//! version. The CI auto-bump workflow (`.github/workflows/auto-version-bump.yml`)
//! appends a `+sha.<short-sha>` build-metadata suffix on every push to
//! main, so we accept any plugin version of the form
//! `<base>` or `<base>+...` or `<base>-...` as long as `<base>` matches
//! the workspace version verbatim.
//!
//! If this test fails, bump `plugin/.claude-plugin/plugin.json`'s
//! `version` field to match `Cargo.toml`'s `[workspace.package].version`.
//! CI will refresh the SHA suffix on the next push to main.

use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn read_workspace_version() -> String {
    let cargo_toml =
        std::fs::read_to_string(workspace_root().join("Cargo.toml")).expect("read Cargo.toml");
    let mut in_workspace_package = false;
    for line in cargo_toml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_workspace_package = trimmed == "[workspace.package]";
            continue;
        }
        if in_workspace_package {
            if let Some(rest) = trimmed.strip_prefix("version = ") {
                return rest.trim_matches('"').to_string();
            }
        }
    }
    panic!("[workspace.package].version not found in Cargo.toml")
}

fn read_plugin_version() -> String {
    let plugin_json_path = workspace_root().join("plugin/.claude-plugin/plugin.json");
    let plugin_json = std::fs::read_to_string(&plugin_json_path).expect("read plugin.json");
    let v: serde_json::Value =
        serde_json::from_str(&plugin_json).expect("plugin.json must parse as JSON");
    v.get("version")
        .and_then(|v| v.as_str())
        .expect("plugin.json must have a string `version` field")
        .to_string()
}

#[test]
fn plugin_version_base_matches_workspace_version() {
    let workspace = read_workspace_version();
    let plugin = read_plugin_version();

    // Strip any semver build metadata (`+sha.…`) or pre-release suffix
    // (`-rc.1`); we only enforce the base part.
    let plugin_base = plugin
        .split_once('+')
        .map(|(b, _)| b)
        .or_else(|| plugin.split_once('-').map(|(b, _)| b))
        .unwrap_or(&plugin);

    assert_eq!(
        plugin_base, workspace,
        "plugin.json version base ({plugin_base}, full: {plugin}) must match Cargo.toml \
         workspace version ({workspace}). When bumping the workspace version, also bump \
         plugin.json's version base — CI auto-appends the SHA suffix on push."
    );
}
