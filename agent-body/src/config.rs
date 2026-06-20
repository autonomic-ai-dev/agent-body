use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub logging: LoggingConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            logging: LoggingConfig {
                level: "info".into(),
            },
        }
    }
}

impl Config {
    pub fn config_path() -> std::path::PathBuf {
        agent_body_core::config_path()
    }

    pub fn load() -> Result<Self> {
        agent_body_core::organ_config::load("autonomic")
    }
}
