use crate::common::constants::{CONSTRUCTORS, DESTRUCTORS};
use crate::common::types::pb;
use crate::common::types::pb::Operation;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainState {
    terrain_name: String,
    biome_name: String,
    toml_path: String,
    timestamp: String,
    execute_request: ExecuteRequest,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExecuteRequest {
    operation: String,
    commands_state: Vec<CommandState>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandState {
    pid: u32,
    command: Command,
    log_path: String,
    exit_code: i32,
    status: CommandStatus,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Command {
    exe: String,
    args: Vec<String>,
    env: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CommandStatus {
    Initialized,
    Running,
    Failed(i32),
    Succeeded,
}

impl TerrainState {
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(&self).context("Failed to serialize TerrainState")
    }
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to deserialize TerrainState")
    }
}

impl From<pb::ActivateRequest> for TerrainState {
    fn from(value: pb::ActivateRequest) -> Self {
        let execute_request: ExecuteRequest = value.execute.unwrap().into();
        Self {
            terrain_name: value.terrain_name,
            biome_name: value.biome_name,
            toml_path: value.toml_path,
            timestamp: value.timestamp,
            execute_request,
        }
    }
}

impl From<pb::ExecuteRequest> for ExecuteRequest {
    fn from(value: pb::ExecuteRequest) -> Self {
        let operation = Operation::try_from(value.operation).expect("invalid operation");

        let op = match operation {
            Operation::Unspecified => "unspecified",
            Operation::Constructors => CONSTRUCTORS,
            Operation::Destructors => DESTRUCTORS,
        }
        .to_string();

        let commands_state: Vec<CommandState> = value
            .commands
            .into_iter()
            .map(|command| CommandState {
                pid: u32::MAX,
                command: Command {
                    exe: command.exe,
                    args: command.args,
                    env: command.envs,
                },
                log_path: "".to_string(),
                exit_code: i32::MAX,
                status: CommandStatus::Initialized,
            })
            .collect();

        Self {
            operation: op,
            commands_state,
        }
    }
}
