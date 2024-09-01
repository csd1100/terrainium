use std::{collections::HashMap, path::PathBuf};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::proto::{self, ActivateRequest};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ClientStatus {
    pub(crate) terrain_name: String,
    pub(crate) biome_name: String,
    pub(crate) toml_path: PathBuf,
    pub(crate) session_id: String,
    pub(crate) terrain_status: TerrainStatus,
    pub(crate) process_status: HashMap<u32, ProcessStatus>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum TerrainStatus {
    UNSPECIFIED,
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

fn session_id_from_history(history: i32) -> Result<String> {
    match proto::request::History::try_from(history)? {
        proto::request::History::Unspecified => {
            Err(anyhow!("invalid session history parameter found"))
        }
        proto::request::History::Last => Ok("last".to_string()),
        proto::request::History::Last1 => Ok("last1".to_string()),
        proto::request::History::Last2 => Ok("last2".to_string()),
    }
}

pub fn session_id_from(request: &proto::Request) -> Result<String> {
    match &request.session {
        Some(session) => match session {
            proto::request::Session::SessionId(session_id) => Ok(session_id.clone()),
            proto::request::Session::History(history) => session_id_from_history(*history),
        },
        None => Err(anyhow!("no session was found in request")),
    }
}

pub fn get_status_for_session(session_id: String, request: ActivateRequest) -> ClientStatus {
    let mut status: ClientStatus = request.into();
    status.session_id = session_id;
    status
}

impl From<ActivateRequest> for ClientStatus {
    fn from(value: ActivateRequest) -> Self {
        Self {
            terrain_name: value.terrain_name,
            biome_name: value.biome_name,
            toml_path: PathBuf::new(),
            session_id: String::new(),
            terrain_status: TerrainStatus::ACTIVE,
            process_status: HashMap::new(),
        }
    }
}

impl TryFrom<TerrainStatus> for i32 {
    type Error = anyhow::Error;

    fn try_from(val: TerrainStatus) -> Result<Self, Self::Error> {
        match val {
            TerrainStatus::ACTIVE => Ok(proto::status_response::TerrainStatus::from_str_name(
                "TERRAIN_STATUS_ACTIVE",
            )
            .expect("to be converted")
            .into()),
            TerrainStatus::INACTIVE => Ok(proto::status_response::TerrainStatus::from_str_name(
                "TERRAIN_STATUS_INACTIVE",
            )
            .expect("to be converted")
            .into()),
            TerrainStatus::UNSPECIFIED => Err(anyhow!("terrain status cannot be UNSPECIFIED")),
        }
    }
}

impl From<CommandStatus> for i32 {
    fn from(val: CommandStatus) -> Self {
        match val {
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

impl From<ProcessStatus> for proto::status_response::ProcessStatus {
    fn from(val: ProcessStatus) -> Self {
        proto::status_response::ProcessStatus {
            pid: val.pid,
            uuid: val.uuid,
            command: val.cmd,
            args: val.args,
            status: val.status.into(),
            stdout_file_path: val.stdout_file.to_string_lossy().to_string(),
            stderr_file_path: val.stderr_file.to_string_lossy().to_string(),
            exit_code: val.ec,
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

impl TryInto<proto::StatusResponse> for ClientStatus {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<proto::StatusResponse, Self::Error> {
        if self.session_id.is_empty() {
            return Err(anyhow!("session_id not set on status object"));
        }
        Ok(proto::StatusResponse {
            response: Some(proto::status_response::Response::Status(
                proto::status_response::Status {
                    session_id: self.session_id,
                    terrain_name: self.terrain_name,
                    biome_name: self.biome_name,
                    toml_path: self.toml_path.to_string_lossy().to_string(),
                    status: self.terrain_status.try_into()?,
                    ps: get_process_map(self.process_status),
                },
            )),
        })
    }
}
