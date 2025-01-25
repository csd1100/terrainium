use crate::common::constants::{CONSTRUCTORS, DESTRUCTORS, TERRAINIUMD_TMP_DIR};
use crate::common::run::CommandToRun;
use crate::common::types::pb;
use anyhow::{anyhow, Context, Result};
use owo_colors::{OwoColorize, Style};
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::fmt::{Display, Formatter};
use tokio::fs;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainState {
    session_id: String,
    terrain_name: String,
    biome_name: String,
    toml_path: String,
    start_timestamp: String,
    end_timestamp: String,
    is_activate: bool,
    execute_context: ExecutionContext,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExecutionContext {
    constructors_state: Vec<CommandState>,
    destructors_state: Vec<CommandState>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandState {
    operation: String,
    command: CommandToRun,
    log_path: String,
    status: CommandStatus,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CommandStatus {
    Starting,
    Running,
    Failed(Option<i32>),
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
        self.end_timestamp.as_str()
    }

    pub fn execute_context(&self) -> &ExecutionContext {
        &self.execute_context
    }

    pub(crate) fn execute_context_mut(&mut self) -> &mut ExecutionContext {
        &mut self.execute_context
    }

    pub fn set_log_path(&mut self, idx: usize, operation: &str) {
        if operation == CONSTRUCTORS {
            self.execute_context
                .constructors_state
                .get_mut(idx)
                .expect("to be present")
                .log_path = format!(
                "{}/{}.{}.{}.log",
                self.dir_path(),
                operation,
                idx,
                self.start_timestamp.as_str()
            );
        } else {
            self.execute_context
                .destructors_state
                .get_mut(idx)
                .expect("to be present")
                .log_path = format!(
                "{}/{}.{}.{}.log",
                self.dir_path(),
                operation,
                idx,
                self.end_timestamp.as_str()
            );
        }
    }

    pub fn dir_path(&self) -> String {
        let identifier: &str = if !self.session_id.is_empty() {
            &self.session_id
        } else if !self.start_timestamp.is_empty() {
            &self.start_timestamp
        } else {
            &self.end_timestamp
        };
        format!(
            "{}/{}/{}",
            TERRAINIUMD_TMP_DIR, self.terrain_name, identifier
        )
    }

    pub fn file_path(&self) -> String {
        format!("{}/state.json", self.dir_path())
    }

    pub(crate) fn merge(&mut self, other: Self) -> Result<()> {
        if self.session_id != other.session_id {
            return Err(anyhow!("cannot merge unrelated terrain states"));
        }

        self.end_timestamp = other.end_timestamp;
        self.execute_context.destructors_state = other.execute_context.destructors_state;

        Ok(())
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

    pub(crate) async fn readable_file(&self) -> Result<fs::File> {
        fs::File::options()
            .read(true)
            .open(self.file_path())
            .await
            .context("Failed to read TerrainState file")
    }

    pub(crate) fn rendered(&self, json: bool) -> String {
        if json {
            self.to_json().expect("Failed to serialize TerrainState")
        } else {
            format!("\n{}", self)
        }
    }
}

impl ExecutionContext {
    pub fn commands(&self, operation: &str) -> Vec<CommandToRun> {
        if operation == CONSTRUCTORS {
            self.constructors_state
                .iter()
                .map(|state| state.command.clone())
                .collect()
        } else {
            self.destructors_state
                .iter()
                .map(|state| state.command.clone())
                .collect()
        }
    }

    pub fn command(&self, idx: usize, operation: &str) -> &CommandToRun {
        if operation == CONSTRUCTORS {
            &self
                .constructors_state
                .get(idx)
                .expect("to be present")
                .command
        } else {
            &self
                .destructors_state
                .get(idx)
                .expect("to be present")
                .command
        }
    }

    pub fn log_path(&self, idx: usize, operation: &str) -> &str {
        if operation == CONSTRUCTORS {
            self.constructors_state
                .get(idx)
                .expect("to be present")
                .log_path
                .as_str()
        } else {
            self.destructors_state
                .get(idx)
                .expect("to be present")
                .log_path
                .as_str()
        }
    }

    pub fn set_command_state(
        &mut self,
        idx: usize,
        operation: &str,
        command_status: CommandStatus,
    ) {
        if operation == CONSTRUCTORS {
            self.constructors_state
                .get_mut(idx)
                .expect("to be present")
                .status = command_status;
        } else {
            self.destructors_state
                .get_mut(idx)
                .expect("to be present")
                .status = command_status;
        }
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(&self).context("Failed to serialize ExecuteRequest")
    }

    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).context("Failed to deserialize ExecuteRequest")
    }
}

impl CommandState {
    pub fn operation(&self) -> &str {
        self.operation.as_str()
    }
}

pub fn operation_name(operation: i32) -> String {
    let operation = pb::Operation::try_from(operation).expect("invalid operation");
    match operation {
        pb::Operation::Unspecified => "unspecified",
        pb::Operation::Constructors => CONSTRUCTORS,
        pb::Operation::Destructors => DESTRUCTORS,
    }
    .to_string()
}

impl From<pb::ExecuteRequest> for TerrainState {
    fn from(value: pb::ExecuteRequest) -> Self {
        let execution_context = value.clone().into();

        let (start_time, end_time) = if operation_name(value.operation) == CONSTRUCTORS {
            (value.timestamp, "".to_string())
        } else {
            ("".to_string(), value.timestamp)
        };

        Self {
            session_id: value.session_id,
            terrain_name: value.terrain_name,
            biome_name: value.biome_name,
            toml_path: value.toml_path,
            start_timestamp: start_time,
            end_timestamp: end_time,
            is_activate: value.is_activate,
            execute_context: execution_context,
        }
    }
}

impl From<pb::ExecuteRequest> for ExecutionContext {
    fn from(value: pb::ExecuteRequest) -> Self {
        let commands_state: Vec<CommandState> = value
            .commands
            .into_iter()
            .map(|command| CommandState {
                operation: operation_name(value.operation),
                command: CommandToRun::new(command.exe, command.args, Some(command.envs)),
                log_path: "".to_string(),
                status: CommandStatus::Starting,
            })
            .collect();

        if operation_name(value.operation) == CONSTRUCTORS {
            Self {
                constructors_state: commands_state,
                destructors_state: vec![],
            }
        } else {
            Self {
                constructors_state: vec![],
                destructors_state: commands_state,
            }
        }
    }
}

impl From<TerrainState> for pb::StatusResponse {
    fn from(value: TerrainState) -> Self {
        Self {
            session_id: value.session_id,
            terrain_name: value.terrain_name,
            biome_name: value.biome_name,
            toml_path: value.toml_path,
            start_timestamp: value.start_timestamp,
            end_timestamp: value.end_timestamp,
            is_activate: value.is_activate,
            execute_context: Some(value.execute_context.into()),
        }
    }
}

impl From<ExecutionContext> for pb::status_response::ExecutionContext {
    fn from(value: ExecutionContext) -> Self {
        let constructors_state: Vec<pb::status_response::execution_context::CommandState> = value
            .clone()
            .constructors_state
            .into_iter()
            .map(|state| state.into())
            .collect();

        let destructors_state: Vec<pb::status_response::execution_context::CommandState> = value
            .destructors_state
            .into_iter()
            .map(|state| state.into())
            .collect();

        Self {
            constructors_state,
            destructors_state,
        }
    }
}

impl From<CommandState> for pb::status_response::execution_context::CommandState {
    fn from(value: CommandState) -> Self {
        let mut exit_code: i32 = i32::MAX;
        let status: i32 = match value.status {
            CommandStatus::Starting => 1,
            CommandStatus::Running => 2,
            CommandStatus::Failed(v) => {
                exit_code = v.unwrap_or(i32::MAX);
                3
            }
            CommandStatus::Succeeded => 4,
        };

        Self {
            operation: value.operation,
            command: Some(value.command.into()),
            log_path: value.log_path,
            status,
            exit_code,
        }
    }
}

impl From<pb::StatusResponse> for TerrainState {
    fn from(value: pb::StatusResponse) -> Self {
        Self {
            session_id: value.session_id,
            terrain_name: value.terrain_name,
            biome_name: value.biome_name,
            toml_path: value.toml_path,
            start_timestamp: value.start_timestamp,
            end_timestamp: value.end_timestamp,
            is_activate: value.is_activate,
            execute_context: value.execute_context.expect("to be present").into(),
        }
    }
}

impl From<pb::status_response::ExecutionContext> for ExecutionContext {
    fn from(value: pb::status_response::ExecutionContext) -> Self {
        let constructors_state: Vec<CommandState> = value
            .clone()
            .constructors_state
            .into_iter()
            .map(|state| state.into())
            .collect();

        let destructors_state: Vec<CommandState> = value
            .destructors_state
            .into_iter()
            .map(|state| state.into())
            .collect();
        Self {
            constructors_state,
            destructors_state,
        }
    }
}

impl From<pb::status_response::execution_context::CommandState> for CommandState {
    fn from(value: pb::status_response::execution_context::CommandState) -> Self {
        let status = match value.status {
            1 => CommandStatus::Starting,
            2 => CommandStatus::Running,
            3 => CommandStatus::Failed(Some(value.exit_code)),
            4 => CommandStatus::Succeeded,
            _ => panic!("Invalid CommandStatus"),
        };

        Self {
            operation: value.operation,
            command: value.command.expect("to be present").into(),
            log_path: value.log_path,
            status,
        }
    }
}

impl Display for TerrainState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut result = String::new();
        let terrain = format!(
            "{}: {} ({})\n",
            "Terrain".style(Style::new().bright_white()),
            self.terrain_name
                .style(Style::new().bright_magenta().bold()),
            self.biome_name.style(Style::new().magenta().italic())
        );
        let session_id = format!(
            "{}: {}\n",
            "SessionId".style(Style::new().bright_white()),
            self.session_id.style(Style::new().bright_blue())
        );
        let toml_path = format!(
            "{}: {}\n",
            "TOML".style(Style::new().bright_white()),
            self.toml_path.style(Style::new().bright_green())
        );
        let start_timestamp = format!(
            "{}: {}\n",
            "Started".style(Style::new().bright_white()),
            self.start_timestamp.style(Style::new().bright_white())
        );
        let end_timestamp = format!(
            "{}: {}\n",
            "Ended".style(Style::new().bright_white()),
            self.end_timestamp.style(Style::new().bright_white())
        );
        let execution_context = format!("{}", self.execute_context);

        result.push_str(&terrain);
        result.push_str(&session_id);
        result.push_str(&toml_path);
        result.push_str(&start_timestamp);
        result.push_str(&end_timestamp);
        result.push_str(&execution_context);
        write!(f, "{}", result)
    }
}

impl Display for ExecutionContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut result = String::new();

        let constructor_header_style = Style::new().bright_white();
        let constructor_header = "Constructors:\n"
            .style(constructor_header_style)
            .to_string();
        let constructors = self
            .constructors_state
            .iter()
            .fold(String::new(), |mut acc, cd| {
                let _ = write!(&mut acc, "{}", cd);
                acc
            });

        let destructor_header_style = Style::new().bright_white();
        let destructor_header = "Destructors:\n".style(destructor_header_style).to_string();
        let destructors = self
            .destructors_state
            .iter()
            .fold(String::new(), |mut acc, cd| {
                let _ = write!(&mut acc, "{}", cd);
                acc
            });

        result.push_str(&constructor_header);
        result.push_str(&constructors);
        result.push_str(&destructor_header);
        result.push_str(&destructors);

        write!(f, "{}", result)
    }
}

impl Display for CommandState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut result = String::new();
        let command = format!(" {} {}\n", self.status, self.command);
        let logs = format!("  󰘍 {}", self.log_path.style(Style::new().bright_yellow()));

        result.push_str(&command);
        result.push_str(&logs);
        writeln!(f, "{}", result)
    }
}

impl Display for CommandStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            CommandStatus::Starting => "Starting".style(Style::new().bold().white()).to_string(),
            CommandStatus::Running => "Running"
                .style(Style::new().bold().bright_yellow())
                .to_string(),
            CommandStatus::Failed(e) => {
                let err_code = e.unwrap_or_else(|| i32::MAX);
                format!("Failed (exit code: {})", err_code)
                    .style(Style::new().bold().bright_red())
                    .to_string()
            }
            CommandStatus::Succeeded => "Succeeded"
                .style(Style::new().bold().bright_green())
                .to_string(),
        };
        write!(f, "{}", str)
    }
}
