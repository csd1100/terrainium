use crate::common::constants::{CONFIG_LOCATION, TERRAINIUM_CONF};
use anyhow::{anyhow, Context, Result};
use home::home_dir;
#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs::read_to_string;
use std::path::PathBuf;
use tracing::{event, Level};

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

fn get_config_path() -> PathBuf {
    home_dir()
        .expect("to get home directory path")
        .join(CONFIG_LOCATION)
        .join(TERRAINIUM_CONF)
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
        let path = get_config_path();
        event!(Level::INFO, "reading terrainium config from {:?}", path);
        if path.exists() {
            return if let Ok(toml_str) = read_to_string(&path) {
                toml::from_str(&toml_str).context("invalid config")
            } else {
                event!(Level::WARN, "could not read config");
                Err(anyhow!("failed to read config"))
            };
        }
        Err(anyhow!("config file {:?} does not exist", path.display()))
    }

    pub(crate) fn auto_apply(&self) -> bool {
        self.auto_apply
    }
}

#[cfg(test)]
impl Config {
    pub(crate) fn auto_apply_off() -> Self {
        Config {
            auto_apply: false,
            ..Config::default()
        }
    }
}
