use std::fs::{read_to_string, write};
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use home::home_dir;
#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{info, trace};

use crate::common::constants::{CONFIG_LOCATION, TERRAINIUMD_CONF};

#[cfg_attr(feature = "terrain-schema", derive(JsonSchema))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DaemonConfig {
    #[serde(default = "schema_url", rename(serialize = "$schema"))]
    schema: String,

    is_root_allowed: bool,
    history_size: usize,
}

pub fn schema_url() -> String {
    "https://raw.githubusercontent.com/csd1100/terrainium/main/schema/terrainiumd-conf-schema.json"
        .to_string()
}

fn get_config_path() -> PathBuf {
    home_dir()
        .expect("to get home directory path")
        .join(CONFIG_LOCATION)
        .join(TERRAINIUMD_CONF)
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            schema: schema_url(),
            is_root_allowed: false,
            history_size: 5,
        }
    }
}

impl DaemonConfig {
    pub fn from_file() -> Result<Self> {
        let path = get_config_path();
        info!("reading config from {path:?}");
        if path.exists() {
            if let Ok(toml_str) = read_to_string(&path) {
                return toml::from_str(&toml_str).context("invalid config");
            }
        }
        bail!("failed to read configuration file {path:?}")
    }

    pub fn create_file() -> Result<()> {
        let path = get_config_path();
        if path.exists() {
            trace!("config file already exists at path {path:?}");
            return Ok(());
        }
        info!("creating config file at path {path:?}");
        let config = toml::to_string_pretty(&Self::default())
            .expect("default configuration should be parsed");
        write(path, config).context("failed to write configuration file")
    }

    pub fn is_root_allowed(&self) -> bool {
        self.is_root_allowed
    }

    pub fn history_size(&self) -> usize {
        self.history_size
    }
}
