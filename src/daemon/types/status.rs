use std::{collections::HashMap, path::PathBuf};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::proto;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Status {
    terrain_name: String,
    biome_name: String,
    toml_path: PathBuf,
    session_id: String,
    terrain_status: TerrainStatus,
    process_status: HashMap<u32, ProcessStatus>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum TerrainStatus {
    ACTIVE,
    INACTIVE,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ProcessStatus {
    pub pid: u32,
    pub uuid: String,
    pub cmd: String,
    pub args: Vec<String>,
    pub status: CommandStatus,
    pub stdout_file: PathBuf,
    pub stderr_file: PathBuf,
    pub ec: i32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum CommandStatus {
    RUNNING,
    STOPPED,
    ERROR,
}

pub fn status_from(session_id: String, request: proto::ActivateRequest) -> Status {
    let mut status = Status::from(request);
    status.session_id = session_id;
    status
}

impl From<proto::ActivateRequest> for Status {
    fn from(value: proto::ActivateRequest) -> Self {
        Self {
            terrain_name: value.terrain_name,
            biome_name: value.biome_name,
            toml_path: PathBuf::from(value.toml_path),
            session_id: "".to_string(),
            terrain_status: TerrainStatus::ACTIVE,
            process_status: HashMap::new(),
        }
    }
}

impl From<TerrainStatus> for i32 {
    fn from(val: TerrainStatus) -> Self {
        match val {
            TerrainStatus::ACTIVE => {
                proto::status_response::TerrainStatus::from_str_name("TERRAIN_STATUS_ACTIVE")
                    .expect("to be converted")
                    .into()
            }
            TerrainStatus::INACTIVE => {
                proto::status_response::TerrainStatus::from_str_name("TERRAIN_STATUS_INACTIVE")
                    .expect("to be converted")
                    .into()
            }
        }
    }
}

impl Into<i32> for CommandStatus {
    fn into(self) -> i32 {
        match self {
            CommandStatus::RUNNING => {
                proto::status_response::process_status::Status::from_str_name("STATUS_RUNNING")
                    .expect("to be converted")
                    .into()
            }
            CommandStatus::STOPPED => {
                proto::status_response::process_status::Status::from_str_name("STATUS_STOPPED")
                    .expect("to be converted")
                    .into()
            }
            CommandStatus::ERROR => {
                proto::status_response::process_status::Status::from_str_name("STATUS_ERROR")
                    .expect("to be converted")
                    .into()
            }
        }
    }
}

impl Into<proto::status_response::ProcessStatus> for ProcessStatus {
    fn into(self) -> proto::status_response::ProcessStatus {
        proto::status_response::ProcessStatus {
            pid: self.pid,
            uuid: self.uuid,
            command: self.cmd,
            args: self.args,
            status: self.status.into(),
            stdout_file_path: self.stdout_file.to_string_lossy().to_string(),
            stderr_file_path: self.stderr_file.to_string_lossy().to_string(),
            exit_code: self.ec,
        }
    }
}

fn get_process_map(
    processes: HashMap<u32, ProcessStatus>,
) -> HashMap<u32, proto::status_response::ProcessStatus> {
    let mut map = HashMap::<u32, proto::status_response::ProcessStatus>::new();
    processes.into_iter().for_each(|(id, status)| {
        map.insert(id, status.into());
    });
    map
}

impl TryInto<proto::StatusResponse> for Status {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<proto::StatusResponse, Self::Error> {
        if self.session_id == "" {
            return Err(anyhow!("session_id not set on status object"));
        }
        Ok(proto::StatusResponse {
            response: Some(proto::status_response::Response::Status(
                proto::status_response::Status {
                    session_id: self.session_id,
                    terrain_name: self.terrain_name,
                    biome_name: self.biome_name,
                    toml_path: self.toml_path.to_string_lossy().to_string(),
                    status: self.terrain_status.into(),
                    ps: get_process_map(self.process_status),
                },
            )),
        })
    }
}
