use std::collections::BTreeMap;
use std::fmt::Display;

use anyhow::{Context as _, Result, bail};
use clap::builder::styling::AnsiColor;
use serde::{Deserialize, Serialize};

use crate::command::Command;
use crate::pb;
use crate::styles::{colored, error, heading, sub_heading, sub_value, success, value, warning};

#[derive(Serialize, Deserialize)]
/// Stores the status of the terrain in Daemon
pub struct TerrainState {
    session_id: String,
    terrain_name: String,
    biome_name: String,
    toml_path: String,
    terrain_dir: String,
    is_background: bool,
    start_timestamp: String,
    end_timestamp: String,
    envs: BTreeMap<String, String>,
    constructors: BTreeMap<String, Vec<CommandState>>,
    destructors: BTreeMap<String, Vec<CommandState>>,
}

impl Display for TerrainState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"{}  {}({})
{}  {}
{}  {}
{}  {}
{}  {}
{}  {}
{}  {}
{}
  {}
{}
  {}
"#,
            heading("󰛍 terrain"),
            value(&self.terrain_name),
            sub_value(&self.biome_name),
            heading("󰇐 location"),
            value(&self.terrain_dir),
            heading(" TOML"),
            value(&self.toml_path),
            heading("󰻾 session"),
            value(&self.session_id),
            heading("󱑀 started"),
            value(&self.start_timestamp),
            heading("󱑈 ended"),
            value(&self.end_timestamp),
            heading(" background"),
            value(if self.is_background { "yes" } else { "no" }),
            heading(" constructors"),
            command_states_to_display(&self.constructors),
            heading(" destructors"),
            command_states_to_display(&self.destructors),
        )
    }
}

fn command_states_to_display(command_states: &BTreeMap<String, Vec<CommandState>>) -> String {
    command_states
        .iter()
        .map(|(key, value)| {
            let commands: String = value.iter().map(|c| c.to_string()).collect();
            format!("\n    {} {}", sub_heading(key), commands)
        })
        .collect()
}

#[derive(Serialize, Deserialize)]
/// Status and Logging information about a [Command]
pub struct CommandState {
    /// Command to be executed
    command: Command,
    /// Log file location of the command
    log_path: String,
    /// Status of the command
    status: CommandStatus,
}

impl Display for CommandState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"
        
        ├ {}  {} {}
        ├ {}  {}
        ├ {}  {}
        └ {}"#,
            colored("", AnsiColor::BrightGreen),
            value(self.command.exe()),
            value(&self.command.args().join(" ")),
            colored("", AnsiColor::BrightYellow),
            // cwd will be always present
            value(
                self.command
                    .cwd()
                    .as_ref()
                    .map_or("", |wd| wd.to_str().unwrap())
            ),
            colored("", AnsiColor::BrightMagenta),
            sub_value(&self.log_path),
            self.status
        )
    }
}

#[derive(Serialize, Deserialize)]
/// Status of the command on Daemon
pub enum CommandStatus {
    /// Command is yet to run
    Starting,
    /// Command is being executed by daemon
    Running,
    /// Command failed with exit code mentioned
    Failed(Option<i32>),
    /// Command completed successfully
    Succeeded,
}

impl Display for CommandStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandStatus::Starting => {
                write!(f, "{}", value(" starting"))
            }
            CommandStatus::Running => {
                write!(f, "{}", warning("󱍸 running"))
            }
            CommandStatus::Failed(exit_code) => {
                write!(
                    f,
                    "{}",
                    error(&format!(
                        "  failed with exit code {}",
                        exit_code.unwrap_or(-1)
                    ))
                )
            }
            CommandStatus::Succeeded => {
                write!(f, "{}", success("  success"))
            }
        }
    }
}

impl TryFrom<pb::status_response::CommandState> for CommandState {
    type Error = anyhow::Error;

    fn try_from(
        value: pb::status_response::CommandState,
    ) -> std::result::Result<Self, Self::Error> {
        let pb::status_response::CommandState {
            command,
            log_path,
            status,
            exit_code,
        } = value;

        let status = pb::status_response::command_state::CommandStatus::try_from(status)
            .context(format!("failed to convert status {status}"))?;

        let status = match status {
            pb::status_response::command_state::CommandStatus::Unspecified => {
                bail!("unspecified command status")
            }
            pb::status_response::command_state::CommandStatus::Starting => CommandStatus::Starting,
            pb::status_response::command_state::CommandStatus::Running => CommandStatus::Running,
            pb::status_response::command_state::CommandStatus::Failed => {
                CommandStatus::Failed(Some(exit_code))
            }
            pb::status_response::command_state::CommandStatus::Succeeded => {
                CommandStatus::Succeeded
            }
        };

        let command = match command {
            None => {
                bail!("command not found");
            }
            Some(cmd) => cmd.into(),
        };
        Ok(Self {
            command,
            log_path,
            status,
        })
    }
}

fn command_states_from(
    input: BTreeMap<String, pb::status_response::CommandStates>,
) -> Result<BTreeMap<String, Vec<CommandState>>> {
    let mut result = BTreeMap::<String, Vec<CommandState>>::new();

    let res: Result<Vec<_>> = input
        .into_iter()
        .map(|(key, wrapper)| -> Result<_> {
            let res: Result<Vec<CommandState>> = wrapper
                .command_states
                .into_iter()
                .map(|state| state.try_into())
                .collect();
            let res = res.context(format!(
                "failed to convert command states for timestamp: {key}"
            ))?;
            result.insert(key, res);
            Ok(())
        })
        .collect();

    if let Err(e) = res {
        bail!("failed to convert command states: {e}");
    }

    Ok(result)
}

impl TryFrom<pb::StatusResponse> for TerrainState {
    type Error = anyhow::Error;
    fn try_from(value: pb::StatusResponse) -> Result<Self, Self::Error> {
        let pb::StatusResponse {
            session_id,
            terrain_name,
            biome_name,
            terrain_dir,
            toml_path,
            is_background,
            start_timestamp,
            end_timestamp,
            envs,
            constructors,
            destructors,
        } = value;

        let constructors_state = command_states_from(constructors)?;
        let destructors_state = command_states_from(destructors)?;

        Ok(Self {
            session_id,
            terrain_name,
            biome_name,
            terrain_dir,
            toml_path,
            is_background,
            start_timestamp,
            end_timestamp,
            envs,
            constructors: constructors_state,
            destructors: destructors_state,
        })
    }
}
