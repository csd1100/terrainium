use crate::common::constants::{CONSTRUCTORS, DESTRUCTORS};
use crate::common::types::pb;
use crate::common::types::pb::Operation;
use crate::common::utils::timestamp;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainState {
    terrain_name: String,
    biome_name: String,
    toml_path: String,
    timestamp: String,
    execute_context: ExecutionContext,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExecutionContext {
    terrain_name: String,
    biome_name: String,
    operation: String,
    commands_state: Vec<CommandState>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandState {
    command: Command,
    log_path: String,
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
    Starting,
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
    pub fn terrain_name(&self) -> &str {
        self.terrain_name.as_str()
    }
    pub fn timestamp(&self) -> &str {
        self.timestamp.as_str()
    }
    pub fn execute_request(&self) -> &ExecutionContext {
        &self.execute_context
    }
    pub fn execute_request_mut(&mut self) -> &mut ExecutionContext {
        &mut self.execute_context
    }
}

impl ExecutionContext {
    pub fn terrain_name(&self) -> &str {
        self.terrain_name.as_str()
    }
    pub fn operation(&self) -> &str {
        self.operation.as_str()
    }
    pub fn set_command_state(&mut self, idx: usize, command_status: CommandStatus) {
        self.commands_state
            .get_mut(idx)
            .expect("to be present")
            .status = command_status;
    }
    pub fn set_log_path(&mut self, idx: usize, log_path: String) {
        self.commands_state
            .get_mut(idx)
            .expect("to be present")
            .log_path = log_path;
    }
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(&self).context("Failed to serialize ExecuteRequest")
    }
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to deserialize ExecuteRequest")
    }
}

impl From<pb::ActivateRequest> for TerrainState {
    fn from(value: pb::ActivateRequest) -> Self {
        let execute_request: ExecutionContext = value.execute.unwrap().into();
        Self {
            terrain_name: value.terrain_name,
            biome_name: value.biome_name,
            toml_path: value.toml_path,
            timestamp: value.timestamp,
            execute_context: execute_request,
        }
    }
}

impl From<pb::ExecuteRequest> for TerrainState {
    fn from(value: pb::ExecuteRequest) -> Self {
        let execution_context = value.clone().into();
        Self {
            terrain_name: value.terrain_name,
            biome_name: value.biome_name,
            toml_path: "".to_string(),
            timestamp: timestamp(),
            execute_context: execution_context,
        }
    }
}

pub fn operation_name(operation: i32) -> String {
    let operation = Operation::try_from(operation).expect("invalid operation");
    match operation {
        Operation::Unspecified => "unspecified",
        Operation::Constructors => CONSTRUCTORS,
        Operation::Destructors => DESTRUCTORS,
    }
    .to_string()
}

impl From<pb::ExecuteRequest> for ExecutionContext {
    fn from(value: pb::ExecuteRequest) -> Self {
        let commands_state: Vec<CommandState> = value
            .commands
            .into_iter()
            .map(|command| CommandState {
                command: Command {
                    exe: command.exe,
                    args: command.args,
                    env: command.envs,
                },
                log_path: "".to_string(),
                status: CommandStatus::Starting,
            })
            .collect();

        Self {
            terrain_name: value.terrain_name,
            biome_name: value.biome_name,
            operation: operation_name(value.operation),
            commands_state,
        }
    }
}
