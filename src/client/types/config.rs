use std::fs::{read_to_string, write};
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use home::home_dir;
#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::common::constants::{CONFIG_LOCATION, TERRAINIUM_CONF};

#[cfg_attr(feature = "terrain-schema", derive(JsonSchema))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "schema_url", rename(serialize = "$schema"))]
    schema: String,

    auto_apply: bool,
}

pub fn schema_url() -> String {
    "https://raw.githubusercontent.com/csd1100/terrainium/main/schema/terrainium-conf-schema.json"
        .to_string()
}

fn get_config_path() -> Result<PathBuf> {
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
    pub fn from_file() -> Result<Self> {
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

    pub(crate) fn auto_apply(&self) -> bool {
        self.auto_apply
    }

    pub fn create_file() -> Result<()> {
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
    pub(crate) fn auto_apply_off() -> Self {
        Config {
            auto_apply: false,
            ..Config::default()
        }
    }
}
