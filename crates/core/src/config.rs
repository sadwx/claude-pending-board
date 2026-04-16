use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// All user-configurable settings, persisted to `~/.claude/pending/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    /// Cooldown after manual dismiss, in minutes. Range: 1-120.
    pub cooldown_minutes: u32,
    /// Whether the HUD re-shows at cooldown expiry if new items arrived.
    pub reminding_enabled: bool,
    /// Delay (seconds) before auto-hiding after the board goes empty. Range: 0-10.
    pub auto_hide_grace_secs: u32,
    /// Duration (seconds) of the dismiss confirmation countdown. Range: 2-10.
    pub dismiss_countdown_secs: u32,
    /// Skip the dismiss confirmation panel entirely.
    pub skip_dismiss_confirmation: bool,
    /// Default terminal adapter name ("wezterm" or "iterm2").
    pub default_adapter: String,
    /// Saved HUD window position (x, y). None = use tray-anchor default.
    pub hud_position: Option<(i32, i32)>,
    /// Enable debug-level logging.
    pub debug_logging: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cooldown_minutes: 15,
            reminding_enabled: true,
            auto_hide_grace_secs: 2,
            dismiss_countdown_secs: 5,
            skip_dismiss_confirmation: false,
            default_adapter: default_adapter_name(),
            hud_position: None,
            debug_logging: false,
        }
    }
}

fn default_adapter_name() -> String {
    if cfg!(target_os = "macos") {
        "iterm2".to_string()
    } else {
        "wezterm".to_string()
    }
}

impl Config {
    /// Default config file path: `~/.claude/pending/config.toml`
    pub fn default_path() -> PathBuf {
        let home = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".claude").join("pending").join("config.toml")
    }

    /// Load config from a TOML file. Returns default config if the file doesn't
    /// exist or is malformed (with a warning log).
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(config) => config,
                Err(e) => {
                    tracing::warn!(error = %e, path = %path.display(), "malformed config, using defaults");
                    Self::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::info!(path = %path.display(), "config file not found, using defaults");
                Self::default()
            }
            Err(e) => {
                tracing::warn!(error = %e, path = %path.display(), "failed to read config, using defaults");
                Self::default()
            }
        }
    }

    /// Save config to a TOML file atomically (write to .tmp + rename).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(std::io::Error::other)?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, &content)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config_values() {
        let config = Config::default();
        assert_eq!(config.cooldown_minutes, 15);
        assert!(config.reminding_enabled);
        assert_eq!(config.auto_hide_grace_secs, 2);
        assert_eq!(config.dismiss_countdown_secs, 5);
        assert!(!config.skip_dismiss_confirmation);
        assert!(config.hud_position.is_none());
        assert!(!config.debug_logging);
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        let config = Config {
            cooldown_minutes: 30,
            reminding_enabled: false,
            auto_hide_grace_secs: 5,
            dismiss_countdown_secs: 8,
            skip_dismiss_confirmation: true,
            default_adapter: "iterm2".to_string(),
            hud_position: Some((100, 200)),
            debug_logging: true,
        };
        config.save(&path).unwrap();
        let loaded = Config::load(&path);
        assert_eq!(config, loaded);
    }

    #[test]
    fn test_load_missing_file_returns_default() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.toml");
        let config = Config::load(&path);
        assert_eq!(config, Config::default());
    }

    #[test]
    fn test_load_malformed_file_returns_default() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad.toml");
        std::fs::write(&path, "this is not valid toml [[[").unwrap();
        let config = Config::load(&path);
        assert_eq!(config, Config::default());
    }

    #[test]
    fn test_load_partial_config_fills_defaults() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("partial.toml");
        std::fs::write(&path, "cooldown_minutes = 42\n").unwrap();
        let config = Config::load(&path);
        assert_eq!(config.cooldown_minutes, 42);
        assert!(config.reminding_enabled); // default
        assert_eq!(config.auto_hide_grace_secs, 2); // default
    }
}
