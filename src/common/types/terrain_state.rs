use crate::common::constants::{TERRAINIUMD_TMP_DIR, TERRAIN_STATE_FILE_NAME};
use crate::common::execute::CommandToRun;
use crate::common::types::pb;
use crate::common::utils::remove_non_numeric;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use tracing::debug;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerrainState {
    session_id: String,
    terrain_name: String,
    biome_name: String,
    toml_path: String,
    is_background: bool,
    start_timestamp: String,
    end_timestamp: String,
    constructors: BTreeMap<String, Vec<CommandState>>,
    destructors: BTreeMap<String, Vec<CommandState>>,
}

impl TerrainState {
    pub fn get_state_dir(terrain_name: &str, session_id: &str) -> PathBuf {
        PathBuf::from(format!(
            "{TERRAINIUMD_TMP_DIR}/{}/{}",
            terrain_name, session_id
        ))
    }

    pub fn session_id(&self) -> &str {
        self.session_id.as_str()
    }

    pub fn terrain_name(&self) -> &str {
        self.terrain_name.as_str()
    }

    pub fn state_dir(&self) -> PathBuf {
        Self::get_state_dir(self.terrain_name(), self.session_id())
    }

    pub fn state_file(&self) -> PathBuf {
        self.state_dir().join(TERRAIN_STATE_FILE_NAME)
    }

    pub fn log_paths(&self, is_constructor: bool, timestamp: &str) -> Vec<String> {
        let map = if is_constructor {
            &self.constructors
        } else {
            &self.destructors
        };

        map.get(timestamp)
            .unwrap()
            .iter()
            .map(|cst| cst.log_path.clone())
            .collect()
    }

    pub fn get_constructors(&self, timestamp: &str) -> Result<Vec<CommandState>> {
        match self.constructors.get(timestamp) {
            None => {
                bail!("could not find constructor for timestamp {timestamp}");
            }
            Some(constructors) => Ok(constructors.clone()),
        }
    }

    pub fn get_destructors(&self, timestamp: &str) -> Result<Vec<CommandState>> {
        match self.destructors.get(timestamp) {
            None => {
                bail!("could not find destructor for timestamp {timestamp}");
            }
            Some(destructors) => Ok(destructors.clone()),
        }
    }

    pub(crate) fn add_commands_if_necessary(
        &mut self,
        timestamp: &str,
        is_constructor: bool,
        commands: Vec<CommandState>,
    ) {
        let map = if is_constructor {
            &mut self.constructors
        } else {
            &mut self.destructors
        };
        if !map.contains_key(timestamp) {
            map.insert(timestamp.to_string(), commands);
        }
    }

    pub fn update_command_status(
        &mut self,
        is_constructor: bool,
        timestamp: &str,
        index: usize,
        status: CommandStatus,
    ) -> Result<()> {
        let map = if is_constructor {
            &mut self.constructors
        } else {
            &mut self.destructors
        };

        let states = map.get_mut(timestamp).context(format!(
            "command states do not exist for timestamp: {timestamp}"
        ))?;

        let state = states
            .get_mut(index)
            .context(format!("command state does not exist for index: {index}"))?;

        debug!(
            terrain_name = self.terrain_name,
            session_id = self.session_id,
            timestamp = timestamp,
            index = index,
            is_constructor = is_constructor,
            "setting command status to {status:?}"
        );
        state.set_status(status);

        Ok(())
    }

    pub fn update_end_timestamp(&mut self, timestamp: String) {
        debug!(
            terrain_name = self.terrain_name,
            session_id = self.session_id,
            "setting end_timestamp to {timestamp}",
        );
        self.end_timestamp = timestamp
    }
}

impl From<pb::Activate> for TerrainState {
    fn from(value: pb::Activate) -> Self {
        let pb::Activate {
            session_id,
            terrain_name,
            biome_name,
            toml_path,
            is_background,
            start_timestamp,
            constructors,
        } = value;

        let mut constructors_state = BTreeMap::<String, Vec<CommandState>>::new();
        if let Some(constructors) = constructors {
            let command_states: Vec<CommandState> = constructors
                .commands
                .into_iter()
                .enumerate()
                .map(|(index, command)| {
                    CommandState::from(
                        &terrain_name,
                        &session_id,
                        true,
                        index,
                        &remove_non_numeric(&constructors.timestamp),
                        command,
                    )
                })
                .collect();
            constructors_state.insert(constructors.timestamp, command_states);
        }
        Self {
            session_id,
            terrain_name,
            biome_name,
            toml_path,
            is_background,
            start_timestamp,
            end_timestamp: "".to_string(),
            constructors: constructors_state,
            destructors: Default::default(),
        }
    }
}

impl From<pb::Execute> for TerrainState {
    fn from(value: pb::Execute) -> Self {
        let pb::Execute {
            session_id,
            terrain_name,
            biome_name,
            toml_path,
            is_constructor,
            timestamp,
            commands,
        } = value;

        let non_numeric = remove_non_numeric(&timestamp);
        let identifier = session_id.unwrap_or_else(|| non_numeric.clone());

        let mut commands_state = BTreeMap::<String, Vec<CommandState>>::new();
        let states: Vec<CommandState> = commands
            .into_iter()
            .enumerate()
            .map(|(index, command)| CommandState {
                command: command.into(),
                log_path: get_log_path(
                    &terrain_name,
                    &identifier,
                    is_constructor,
                    index,
                    &non_numeric,
                ),
                status: CommandStatus::Starting,
            })
            .collect();
        commands_state.insert(timestamp, states);

        let (constructors, destructors) = if is_constructor {
            (commands_state, BTreeMap::new())
        } else {
            (BTreeMap::new(), commands_state)
        };

        Self {
            session_id: identifier,
            terrain_name,
            biome_name,
            toml_path,
            is_background: false,
            start_timestamp: "".to_string(),
            end_timestamp: "".to_string(),
            constructors,
            destructors,
        }
    }
}

fn command_states_to_proto(
    input: BTreeMap<String, Vec<CommandState>>,
) -> BTreeMap<String, pb::status_response::CommandStates> {
    let mut result = BTreeMap::<String, pb::status_response::CommandStates>::new();

    input.into_iter().for_each(|(key, states)| {
        let states = states.into_iter().map(Into::into).collect();
        let wrapper = pb::status_response::CommandStates {
            command_states: states,
        };
        result.insert(key, wrapper);
    });

    result
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
            toml_path,
            is_background,
            start_timestamp,
            end_timestamp,
            constructors,
            destructors,
        } = value;

        let constructors_state = command_states_from(constructors)?;
        let destructors_state = command_states_from(destructors)?;

        Ok(Self {
            session_id,
            terrain_name,
            biome_name,
            toml_path,
            is_background,
            start_timestamp,
            end_timestamp,
            constructors: constructors_state,
            destructors: destructors_state,
        })
    }
}

impl From<TerrainState> for pb::StatusResponse {
    fn from(value: TerrainState) -> Self {
        let TerrainState {
            session_id,
            terrain_name,
            biome_name,
            toml_path,
            is_background,
            start_timestamp,
            end_timestamp,
            constructors,
            destructors,
        } = value;

        Self {
            session_id,
            terrain_name,
            biome_name,
            toml_path,
            is_background,
            start_timestamp,
            end_timestamp,
            constructors: command_states_to_proto(constructors),
            destructors: command_states_to_proto(destructors),
        }
    }
}

impl From<CommandState> for pb::status_response::CommandState {
    fn from(value: CommandState) -> Self {
        let CommandState {
            command,
            log_path,
            status,
        } = value;

        let (status, exit_code) = match status {
            CommandStatus::Starting => {
                let status = pb::status_response::command_state::CommandStatus::Starting.into();
                (status, -100)
            }
            CommandStatus::Running => {
                let status = pb::status_response::command_state::CommandStatus::Running.into();
                (status, -200)
            }
            CommandStatus::Failed(exit_code) => {
                let status = pb::status_response::command_state::CommandStatus::Failed.into();
                let ec = exit_code.unwrap_or(-99);
                (status, ec)
            }
            CommandStatus::Succeeded => {
                let status = pb::status_response::command_state::CommandStatus::Succeeded.into();
                (status, 0)
            }
        };

        Self {
            command: Some(command.into()),
            log_path,
            status,
            exit_code,
        }
    }
}

fn get_log_path(
    terrain_name: &str,
    identifier: &str,
    is_constructor: bool,
    index: usize,
    numeric_timestamp: &str,
) -> String {
    let operation = if is_constructor {
        "constructor"
    } else {
        "destructor"
    };
    format!(
        "{TERRAINIUMD_TMP_DIR}/{terrain_name}/{identifier}/{operation}.{index}.{numeric_timestamp}.log"
    )
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandState {
    pub(crate) command: CommandToRun,
    pub(crate) log_path: String,
    status: CommandStatus,
}

impl CommandState {
    pub(crate) fn set_status(&mut self, status: CommandStatus) {
        self.status = status;
    }

    pub(crate) fn from(
        terrain_name: &str,
        session_id: &str,
        is_constructor: bool,
        index: usize,
        numeric_timestamp: &str,
        command: pb::Command,
    ) -> Self {
        Self {
            command: command.into(),
            log_path: get_log_path(
                terrain_name,
                session_id,
                is_constructor,
                index,
                numeric_timestamp,
            ),
            status: CommandStatus::Starting,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CommandStatus {
    Starting,
    Running,
    Failed(Option<i32>),
    Succeeded,
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
