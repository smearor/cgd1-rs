use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use serde::Deserialize;
use serde::Serialize;
use tracing::debug;
use tracing::warn;

/// Application configuration persisted to `~/.config/cgd1-rs/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Whether to play the triple-blink visual feedback when connecting.
    #[serde(default = "default_blink_on_connect")]
    pub blink_on_connect: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            blink_on_connect: default_blink_on_connect(),
        }
    }
}

fn default_blink_on_connect() -> bool {
    true
}

/// Thread-safe shared configuration with automatic persistence.
#[derive(Clone)]
pub struct ConfigStore {
    config: Arc<Mutex<AppConfig>>,
    path: PathBuf,
}

impl ConfigStore {
    /// Load config from the default platform config directory.
    ///
    /// Uses `~/.config/cgd1-rs/config.toml` on Linux. If the file does not
    /// exist or is invalid, default values are returned.
    pub fn load_default() -> Self {
        let path = Self::default_path();
        let config = Self::load_from(&path);
        Self {
            config: Arc::new(Mutex::new(config)),
            path,
        }
    }

    /// Get the default config file path.
    fn default_path() -> PathBuf {
        let config = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        config.join("cgd1-rs").join("config.toml")
    }

    /// Load config from a specific path, falling back to defaults on error.
    fn load_from(path: &PathBuf) -> AppConfig {
        match std::fs::read_to_string(path) {
            Ok(content) => match toml::from_str::<AppConfig>(&content) {
                Ok(config) => {
                    debug!(path = %path.display(), "config loaded");
                    config
                }
                Err(e) => {
                    warn!(path = %path.display(), error = %e, "config parse error, using defaults");
                    AppConfig::default()
                }
            },
            Err(_) => {
                debug!(path = %path.display(), "config file not found, using defaults");
                AppConfig::default()
            }
        }
    }

    /// Save the current config to disk.
    pub fn save(&self) {
        let config = self.config.lock().unwrap_or_else(|p| {
            warn!("config mutex poisoned - recovering");
            p.into_inner()
        });
        if let Some(parent) = self.path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                warn!(path = %self.path.display(), error = %e, "failed to create config directory");
                return;
            }
        }
        match toml::to_string_pretty(&*config) {
            Ok(toml_str) => {
                if let Err(e) = std::fs::write(&self.path, toml_str) {
                    warn!(path = %self.path.display(), error = %e, "failed to write config");
                } else {
                    debug!(path = %self.path.display(), "config saved");
                }
            }
            Err(e) => {
                warn!(error = %e, "failed to serialize config");
            }
        }
    }

    /// Get the current config value for blink_on_connect.
    pub fn blink_on_connect(&self) -> bool {
        self.config.lock().unwrap_or_else(|p| {
            warn!("config mutex poisoned - recovering");
            p.into_inner()
        }).blink_on_connect
    }

    /// Set blink_on_connect and persist to disk.
    pub fn set_blink_on_connect(&self, value: bool) {
        {
            let mut config = self.config.lock().unwrap_or_else(|p| {
                warn!("config mutex poisoned - recovering");
                p.into_inner()
            });
            config.blink_on_connect = value;
        }
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_blink_enabled() {
        let config = AppConfig::default();
        assert!(config.blink_on_connect);
    }

    #[test]
    fn config_roundtrip() {
        let dir = std::env::temp_dir().join("cgd1_test_config_roundtrip");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("config.toml");

        let store = ConfigStore {
            config: Arc::new(Mutex::new(AppConfig {
                blink_on_connect: false,
            })),
            path: path.clone(),
        };
        store.save();

        let loaded = ConfigStore::load_from(&path);
        assert!(!loaded.blink_on_connect);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_uses_defaults() {
        let path = std::env::temp_dir().join("cgd1_test_config_nonexistent.toml");
        let _ = std::fs::remove_file(&path);
        let loaded = ConfigStore::load_from(&path);
        assert!(loaded.blink_on_connect);
    }
}
