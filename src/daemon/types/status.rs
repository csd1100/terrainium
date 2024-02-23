use anyhow::{anyhow, Context, Result};
use std::{
    collections::HashMap,
    fs::File,
    ops::Deref,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct DaemonStatus {
    pub active_terrains: HashMap<String, ActiveTerrain>,
    pub history: [String; 3],
}

impl DaemonStatus {
    pub fn new() -> Self {
        DaemonStatus {
            active_terrains: HashMap::new(),
            history: ["".to_string(), "".to_string(), "".to_string()],
        }
    }
}

impl Default for DaemonStatus {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ActiveTerrain {
    pub name: String,
    pub biome: String,
    pub toml: PathBuf,
    pub status_file: PathBuf,
}

pub fn status_from(daemon_status_file: Arc<Mutex<File>>) -> Result<DaemonStatus> {
    let res = daemon_status_file.lock();
    let file = match res {
        Ok(file) => file,
        Err(err) => {
            return Err(anyhow!(
                "error while acquiring a lock on daemon status file {}",
                err
            ))
        }
    };
    let daemon_status: DaemonStatus =
        serde_json::from_reader(file.deref()).context("error parsing daemon status file")?;
    Ok(daemon_status)
}

pub fn status_to(daemon_status_file: Arc<Mutex<File>>, status: DaemonStatus) -> Result<()> {
    let res = daemon_status_file.lock();
    let file = match res {
        Ok(file) => file,
        Err(err) => {
            return Err(anyhow!(
                "error while acquiring a lock on daemon status file {}",
                err
            ))
        }
    };

    serde_json::to_writer_pretty(file.deref(), &status)
        .context("error writing daemon status object to json")?;
    Ok(())
}
