use crate::common::constants::TERRAINIUMD_TMP_DIR;
use crate::common::execute::CommandToRun;
use crate::common::types::pb;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainState {
    session_id: String,
    terrain_name: String,
    biome_name: String,
    toml_path: String,
    is_background: bool,
    start_timestamp: String,
    end_timestamp: String,
    constructors: HashMap<String, Vec<CommandState>>,
    destructors: HashMap<String, Vec<CommandState>>,
}

impl TerrainState {
    pub fn session_id(&self) -> &str {
        self.session_id.as_str()
    }

    pub fn terrain_name(&self) -> &str {
        self.terrain_name.as_str()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandState {
    command: CommandToRun,
    log_path: String,
    status: CommandStatus,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CommandStatus {
    Initializing,
    Starting,
    Running,
    Failed(Option<i32>),
    Succeeded,
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
                .map(|(index, command)| CommandState {
                    command: command.into(),
                    log_path: format!(
                        "{TERRAINIUMD_TMP_DIR}/{terrain_name}/{session_id}/constructor.{index}.log"
                    ),
                    status: CommandStatus::Initializing,
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
