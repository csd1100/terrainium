use crate::common::constants::{CONSTRUCTORS, DESTRUCTORS, TERRAINIUMD_TMP_DIR};
use crate::common::execute::CommandToRun;
use crate::common::types::pb;
use crate::common::types::pb::Operation;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainState {
    terrain_name: String,
    biome_name: String,
    toml_path: String,
    timestamp: String,
    is_activate: bool,
    execute_context: ExecutionContext,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExecutionContext {
    operation: String,
    commands_state: Vec<CommandState>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandState {
    command: CommandToRun,
    log_path: String,
    status: CommandStatus,
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

    pub fn execute_context(&self) -> &ExecutionContext {
        &self.execute_context
    }

    pub(crate) fn execute_context_mut(&mut self) -> &mut ExecutionContext {
        &mut self.execute_context
    }

    pub fn set_log_path(&mut self, idx: usize) {
        self.execute_context
            .commands_state
            .get_mut(idx)
            .expect("to be present")
            .log_path = format!(
            "{}/{}.{}.{}.log",
            self.dir_path(),
            self.execute_context.operation.as_str(),
            idx,
            self.timestamp.as_str()
        );
    }

    pub fn dir_path(&self) -> String {
        format!(
            "{}/{}/{}",
            TERRAINIUMD_TMP_DIR, self.terrain_name, self.timestamp
        )
    }

    pub fn file_path(&self) -> String {
        format!("{}/state.json", self.dir_path())
    }

    pub(crate) async fn new_file(&self) -> Result<fs::File> {
        fs::File::options()
            .create(true)
            .truncate(true)
            .write(true)
            .open(self.file_path())
            .await
            .context("Failed to create TerrainState file")
    }

    pub(crate) async fn writable_file(&self) -> Result<fs::File> {
        fs::File::options()
            .truncate(true)
            .write(true)
            .open(self.file_path())
            .await
            .context("Failed to create TerrainState file")
    }

    // pub(crate) async fn readable_file(&self) -> Result<fs::File> {
    //     fs::File::options()
    //         .truncate(true)
    //         .write(true)
    //         .open(self.file_path())
    //         .await
    //         .context("Failed to create TerrainState file")
    // }
}

impl ExecutionContext {
    pub fn operation(&self) -> &str {
        self.operation.as_str()
    }

    pub fn commands(&self) -> Vec<CommandToRun> {
        self.commands_state
            .iter()
            .map(|state| state.command.clone())
            .collect()
    }

    pub fn command(&self, idx: usize) -> &CommandToRun {
        &self.commands_state.get(idx).expect("to be present").command
    }

    pub fn log_path(&self, idx: usize) -> &str {
        self.commands_state
            .get(idx)
            .expect("to be present")
            .log_path
            .as_str()
    }

    pub fn set_command_state(&mut self, idx: usize, command_status: CommandStatus) {
        self.commands_state
            .get_mut(idx)
            .expect("to be present")
            .status = command_status;
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(&self).context("Failed to serialize ExecuteRequest")
    }

    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to deserialize ExecuteRequest")
    }
}

impl From<pb::ExecuteRequest> for TerrainState {
    fn from(value: pb::ExecuteRequest) -> Self {
        let execution_context = value.clone().into();
        Self {
            terrain_name: value.terrain_name,
            biome_name: value.biome_name,
            toml_path: value.toml_path,
            timestamp: value.timestamp,
            is_activate: value.is_activate,
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
                command: CommandToRun::new(command.exe, command.args, Some(command.envs)),
                log_path: "".to_string(),
                status: CommandStatus::Starting,
            })
            .collect();

        Self {
            operation: operation_name(value.operation),
            commands_state,
        }
    }
}
