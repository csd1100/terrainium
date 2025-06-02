use crate::common::constants::{TERRAINIUMD_TMP_DIR, TERRAIN_STATE_FILE_NAME};
use crate::common::execute::CommandToRun;
use crate::common::types::pb;
use crate::common::utils::remove_non_numeric;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::debug;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainState {
    pub(crate) session_id: String,
    pub(crate) terrain_name: String,
    pub(crate) biome_name: String,
    pub(crate) toml_path: String,
    pub(crate) is_background: bool,
    pub(crate) start_timestamp: String,
    pub(crate) end_timestamp: String,
    pub(crate) constructors: HashMap<String, Vec<CommandState>>,
    pub(crate) destructors: HashMap<String, Vec<CommandState>>,
}

impl TerrainState {
    pub fn from_construct_destruct(
        session_id: Option<String>,
        terrain_name: String,
        biome_name: String,
        toml_path: String,
        timestamp: String,
        is_construct: bool,
        commands: Vec<pb::Command>,
    ) -> Self {
        let non_numeric = remove_non_numeric(&timestamp);
        let identifier = session_id.unwrap_or_else(|| non_numeric.clone());

        let mut commands_state = HashMap::<String, Vec<CommandState>>::new();
        let states: Vec<CommandState> = commands
            .into_iter()
            .enumerate()
            .map(|(index, command)| CommandState {
                command: command.into(),
                log_path: get_log_path(&terrain_name, &identifier, index, &non_numeric),
                status: CommandStatus::Starting,
            })
            .collect();
        commands_state.insert(timestamp, states);

        let (constructors, destructors) = if is_construct {
            (commands_state, HashMap::new())
        } else {
            (HashMap::new(), commands_state)
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

    pub fn session_id(&self) -> &str {
        self.session_id.as_str()
    }

    pub fn terrain_name(&self) -> &str {
        self.terrain_name.as_str()
    }

    pub fn state_dir(&self) -> PathBuf {
        PathBuf::from(format!(
            "{TERRAINIUMD_TMP_DIR}/{}/{}",
            self.terrain_name, self.session_id
        ))
    }

    pub fn state_file(&self) -> PathBuf {
        self.state_dir().join(TERRAIN_STATE_FILE_NAME)
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

        let mut constructors_state = HashMap::<String, Vec<CommandState>>::new();
        if let Some(constructors) = constructors {
            let command_states: Vec<CommandState> = constructors
                .commands
                .into_iter()
                .enumerate()
                .map(|(index, command)| {
                    CommandState::from(
                        &terrain_name,
                        &session_id,
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

fn get_log_path(
    terrain_name: &str,
    identifier: &str,
    index: usize,
    numeric_timestamp: &str,
) -> String {
    format!(
        "{TERRAINIUMD_TMP_DIR}/{terrain_name}/{identifier}/constructor.{index}.{numeric_timestamp}.log"
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
        index: usize,
        numeric_timestamp: &str,
        command: pb::Command,
    ) -> Self {
        Self {
            command: command.into(),
            log_path: get_log_path(terrain_name, session_id, index, numeric_timestamp),
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
