use serde::Deserialize;
use serde::Serialize;

use crate::config::TimeBasedBlink;

/// Application configuration persisted to `~/.config/cgd1-rs/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Whether to play the triple-blink visual feedback when connecting.
    #[serde(default = "default_blink_on_connect")]
    pub blink_on_connect: bool,
    /// Frequency of time-based visual blink feedback.
    #[serde(default)]
    pub time_based_blink: TimeBasedBlink,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            blink_on_connect: default_blink_on_connect(),
            time_based_blink: TimeBasedBlink::default(),
        }
    }
}

fn default_blink_on_connect() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_blink_enabled() {
        let config = AppConfig::default();
        assert!(config.blink_on_connect);
    }
}
