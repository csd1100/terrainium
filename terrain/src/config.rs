use std::fs::{read_to_string, write};
use std::path::PathBuf;

use anyhow::{Context, bail};
use home::home_dir;
use serde::{Deserialize, Serialize};
use terrainium_lib::constants::CONFIG_LOCATION;
use tracing::{error, info};

use crate::constants::TERRAINIUM_CONF;

#[derive(Serialize, Deserialize)]
/// Configuration for terrainium client(`terrain`)
pub struct Config {
    /// JSON schema for this configuration
    #[serde(default = "schema_url", rename(serialize = "$schema"))]
    schema: String,

    /// Globally enable auto_apply mechanism
    ///
    /// default: true
    auto_apply: bool,
}

/// JSON schema url
pub fn schema_url() -> String {
    "https://raw.githubusercontent.com/csd1100/terrainium/main/schema/terrainium-conf-schema.json"
        .to_string()
}

/// Get configuration location.
///
/// Location: `~/.config/terrainium/terrainium.toml`
fn get_config_path() -> anyhow::Result<PathBuf> {
    let home_dir = home_dir().context("to get home directory path")?;
    Ok(home_dir.join(CONFIG_LOCATION).join(TERRAINIUM_CONF))
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema: schema_url(),
            auto_apply: true,
        }
    }
}

impl Config {
    /// Reads config from file and returns [Self]
    pub fn from_file() -> anyhow::Result<Self> {
        let path = get_config_path().context("failed to get config path")?;
        info!("reading terrainium config from {path:?}");
        if path.exists() {
            return if let Ok(toml_str) = read_to_string(&path) {
                toml::from_str(&toml_str).context("invalid config")
            } else {
                error!("could not read config");
                bail!("failed to read config")
            };
        }
        info!("terrainium config does not exist");
        bail!("config file {path:?} does not exist")
    }

    /// Get auto apply value.
    pub(crate) fn auto_apply(&self) -> bool {
        self.auto_apply
    }

    /// Creates the default configuration at file.
    ///
    /// Created at Location: `~/.config/terrainium/terrainium.toml`
    pub fn create_file() -> anyhow::Result<()> {
        let path = get_config_path().context("failed to get config path")?;
        if path.exists() {
            info!("config file already exists at path {path:?}");
            return Ok(());
        }
        info!("creating config file at path {path:?}");
        let config = toml::to_string_pretty(&Self::default())
            .expect("default configuration should be parsed");
        write(path, config).context("failed to write configuration file")
    }
}

#[cfg(test)]
impl Config {
    #[cfg(test)]
    /// Get configuration object with auto_apply set to off
    pub(crate) fn auto_apply_off() -> Self {
        Config {
            auto_apply: false,
            ..Config::default()
        }
    }
}
