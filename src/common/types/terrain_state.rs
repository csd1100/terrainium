use crate::common::constants::{CONSTRUCTORS, DESTRUCTORS, TERRAIN_STATE_FILE_NAME};
use crate::common::types::command::Command;
use crate::common::types::paths::get_terrainiumd_paths;
use crate::common::types::pb;
use crate::common::types::styles::{
    colored, error, heading, sub_heading, sub_value, success, value, warning,
};
use crate::common::utils::remove_non_numeric;
use anyhow::{Context, Result, bail};
use clap::builder::styling::AnsiColor;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use tracing::{debug, instrument, trace};

fn get_log_path(
    state_directory: &str,
    terrain_name: &str,
    identifier: &str,
    is_constructor: bool,
    index: usize,
    numeric_timestamp: &str,
) -> String {
    let operation = if is_constructor {
        CONSTRUCTORS
    } else {
        DESTRUCTORS
    };
    format!(
        "{state_directory}/{terrain_name}/{identifier}/{operation}.{index}.{numeric_timestamp}.log"
    )
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

fn command_states_to_display(command_states: &BTreeMap<String, Vec<CommandState>>) -> String {
    command_states
        .iter()
        .map(|(key, value)| {
            let commands: String = value.iter().map(|c| c.to_string()).collect();
            format!("\n    {} {}", sub_heading(key), commands)
        })
        .collect()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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

impl TerrainState {
    pub fn get_state_dir(state_directory: &str, terrain_name: &str, session_id: &str) -> PathBuf {
        PathBuf::from(format!("{state_directory}/{terrain_name}/{session_id}"))
    }

    pub fn session_id(&self) -> &str {
        self.session_id.as_str()
    }

    pub fn terrain_name(&self) -> &str {
        self.terrain_name.as_str()
    }

    pub fn state_dir(&self, state_directory: &str) -> PathBuf {
        Self::get_state_dir(state_directory, self.terrain_name(), self.session_id())
    }

    pub fn state_file(&self, state_directory: &str) -> PathBuf {
        self.state_dir(state_directory)
            .join(TERRAIN_STATE_FILE_NAME)
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

    pub fn envs(&self) -> BTreeMap<String, String> {
        self.envs.clone()
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

    #[instrument]
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
            trace!("adding the command to state");
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

impl Display for TerrainState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandState {
    command: Command,
    log_path: String,
    status: CommandStatus,
}

impl CommandState {
    pub(crate) fn from(
        state_directory: &str,
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
                state_directory,
                terrain_name,
                session_id,
                is_constructor,
                index,
                numeric_timestamp,
            ),
            status: CommandStatus::Starting,
        }
    }

    pub(crate) fn command_and_log_path(self) -> (Command, String) {
        (self.command, self.log_path)
    }

    pub(crate) fn set_status(&mut self, status: CommandStatus) {
        self.status = status;
    }
}

impl Display for CommandState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
            colored("", AnsiColor::BrightWhite),
            sub_value(&self.log_path),
            self.status
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CommandStatus {
    Starting,
    Running,
    Failed(Option<i32>),
    Succeeded,
}

impl Display for CommandStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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

impl From<pb::Activate> for TerrainState {
    fn from(value: pb::Activate) -> Self {
        let pb::Activate {
            session_id,
            terrain_name,
            biome_name,
            terrain_dir,
            toml_path,
            is_background,
            start_timestamp,
            constructors,
        } = value;

        let envs: BTreeMap<String, String>;

        let mut constructors_state = BTreeMap::<String, Vec<CommandState>>::new();
        if let Some(constructors) = constructors {
            envs = constructors.envs;
            let command_states: Vec<CommandState> = constructors
                .commands
                .into_iter()
                .enumerate()
                .map(|(index, command)| {
                    CommandState::from(
                        get_terrainiumd_paths().dir_str(),
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
        } else {
            envs = BTreeMap::new();
        }
        Self {
            session_id,
            terrain_name,
            biome_name,
            terrain_dir,
            toml_path,
            is_background,
            start_timestamp,
            end_timestamp: "".to_string(),
            envs,
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
            terrain_dir,
            toml_path,
            is_constructor,
            timestamp,
            envs,
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
                    get_terrainiumd_paths().dir_str(),
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
            terrain_dir,
            toml_path,
            is_background: false,
            start_timestamp: "".to_string(),
            end_timestamp: "".to_string(),
            envs,
            constructors,
            destructors,
        }
    }
}

impl TryFrom<Box<pb::StatusResponse>> for TerrainState {
    type Error = anyhow::Error;
    fn try_from(value: Box<pb::StatusResponse>) -> Result<Self, Self::Error> {
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
        } = *value;

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

impl From<TerrainState> for pb::StatusResponse {
    fn from(value: TerrainState) -> Self {
        let TerrainState {
            session_id,
            terrain_name,
            biome_name,
            toml_path,
            terrain_dir,
            is_background,
            start_timestamp,
            end_timestamp,
            envs,
            constructors,
            destructors,
        } = value;

        Self {
            session_id,
            terrain_name,
            biome_name,
            terrain_dir,
            toml_path,
            is_background,
            start_timestamp,
            end_timestamp,
            envs,
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

#[cfg(test)]
pub mod test_utils {
    use super::*;
    use crate::client::test_utils::{
        expected_constructor_background_example_biome, expected_destructor_background_example_biome,
    };
    use crate::client::types::terrain::AutoApply;
    use crate::common::constants::{
        CONSTRUCTORS, DESTRUCTORS, EXAMPLE_BIOME, TERRAIN_TOML, TEST_TIMESTAMP,
    };
    use crate::common::test_utils::{
        TEST_TERRAIN_DIR, TEST_TERRAIN_NAME, TEST_TIMESTAMP_NUMERIC,
        expected_env_vars_example_biome, expected_envs_with_activate_example_biome,
    };
    use std::path::Path;

    fn get_commands(
        state_dir: &str,
        session_id: &str,
        terrain_dir: &str,
        timestamp: &str,
        is_constructor: bool,
        status: CommandStatus,
    ) -> Vec<CommandState> {
        let timestamp = remove_non_numeric(timestamp);
        let mut command_states = vec![];
        let commands = if is_constructor {
            expected_constructor_background_example_biome(Path::new(terrain_dir))
        } else {
            expected_destructor_background_example_biome(Path::new(terrain_dir))
        };

        commands.into_iter().enumerate().for_each(|(idx, command)| {
            command_states.push(CommandState {
                command,
                log_path: format!(
                    "{state_dir}/{TEST_TERRAIN_NAME}/{session_id}/{}.{idx}.{timestamp}.log",
                    if is_constructor {
                        CONSTRUCTORS
                    } else {
                        DESTRUCTORS
                    }
                ),
                status: status.clone(),
            });
        });
        command_states
    }

    fn active_terrain_state_example_biome_with_status(
        state_dir: &str,
        session_id: String,
        is_auto_apply: bool,
        auto_apply: &AutoApply,
        status: CommandStatus,
    ) -> TerrainState {
        let terrain_dir = TEST_TERRAIN_DIR.to_string();
        let toml_path = format!("{terrain_dir}/{TERRAIN_TOML}");

        let mut constructors = BTreeMap::new();
        constructors.insert(
            TEST_TIMESTAMP.to_string(),
            get_commands(
                state_dir,
                &session_id,
                &terrain_dir,
                TEST_TIMESTAMP_NUMERIC,
                true,
                status,
            ),
        );

        TerrainState {
            session_id,
            terrain_name: TEST_TERRAIN_NAME.to_string(),
            biome_name: EXAMPLE_BIOME.to_string(),
            toml_path,
            terrain_dir,
            is_background: true,
            start_timestamp: TEST_TIMESTAMP.to_string(),
            end_timestamp: "".to_string(),
            envs: expected_envs_with_activate_example_biome(is_auto_apply, auto_apply),
            constructors,
            destructors: Default::default(),
        }
    }

    pub fn terrain_state_after_activate(
        session_id: String,
        is_auto_apply: bool,
        auto_apply: &AutoApply,
    ) -> TerrainState {
        active_terrain_state_example_biome_with_status(
            get_terrainiumd_paths().dir_str(),
            session_id,
            is_auto_apply,
            auto_apply,
            CommandStatus::Starting,
        )
    }

    pub fn terrain_state_after_construct(
        session_id: String,
        is_auto_apply: bool,
        auto_apply: &AutoApply,
    ) -> TerrainState {
        active_terrain_state_example_biome_with_status(
            get_terrainiumd_paths().dir_str(),
            session_id,
            is_auto_apply,
            auto_apply,
            CommandStatus::Succeeded,
        )
    }

    pub fn terrain_state_after_construct_failed(
        session_id: String,
        is_auto_apply: bool,
        auto_apply: &AutoApply,
    ) -> TerrainState {
        active_terrain_state_example_biome_with_status(
            get_terrainiumd_paths().dir_str(),
            session_id,
            is_auto_apply,
            auto_apply,
            CommandStatus::Failed(Some(1)),
        )
    }

    fn terrain_state_after_deactivate(
        state_dir: &str,
        session_id: String,
        is_auto_apply: bool,
        auto_apply: &AutoApply,
        status: CommandStatus,
    ) -> TerrainState {
        let mut destructors = BTreeMap::new();
        destructors.insert(
            TEST_TIMESTAMP.to_string(),
            get_commands(
                state_dir,
                &session_id,
                TEST_TERRAIN_DIR,
                TEST_TIMESTAMP,
                false,
                status,
            ),
        );

        let mut state = terrain_state_after_construct(session_id, is_auto_apply, auto_apply);
        state.end_timestamp = TEST_TIMESTAMP.to_string();
        state.destructors = destructors;
        state
    }

    pub fn terrain_state_after_deactivate_before_complete(
        state_dir: &str,
        session_id: String,
        is_auto_apply: bool,
        auto_apply: &AutoApply,
    ) -> TerrainState {
        terrain_state_after_deactivate(
            state_dir,
            session_id,
            is_auto_apply,
            auto_apply,
            CommandStatus::Starting,
        )
    }

    pub fn terrain_state_after_deactivate_after_succeeded(
        state_dir: &str,
        session_id: String,
        is_auto_apply: bool,
        auto_apply: &AutoApply,
    ) -> TerrainState {
        terrain_state_after_deactivate(
            state_dir,
            session_id,
            is_auto_apply,
            auto_apply,
            CommandStatus::Succeeded,
        )
    }

    pub fn terrain_state_after_added_command(
        state_dir: &str,
        session_id: String,
        is_auto_apply: bool,
        auto_apply: &AutoApply,
        is_constructor: bool,
        new_timestamp: String,
    ) -> TerrainState {
        let mut state = terrain_state_after_construct(session_id, is_auto_apply, auto_apply);
        let map = if is_constructor {
            &mut state.constructors
        } else {
            &mut state.destructors
        };
        map.insert(
            new_timestamp.clone(),
            get_commands(
                state_dir,
                &state.session_id,
                TEST_TERRAIN_DIR,
                &new_timestamp,
                is_constructor,
                CommandStatus::Starting,
            ),
        );
        state
    }

    pub fn terrain_state_execute_no_session(
        is_constructor: bool,
        status: CommandStatus,
    ) -> TerrainState {
        let terrain_dir = TEST_TERRAIN_DIR.to_string();
        let toml_path = format!("{terrain_dir}/{TERRAIN_TOML}");

        let commands = if is_constructor {
            expected_constructor_background_example_biome(Path::new(&terrain_dir))
        } else {
            expected_destructor_background_example_biome(Path::new(&terrain_dir))
        };

        let mut command_states = vec![];
        commands.into_iter().enumerate().for_each(|(idx, command)| {
            command_states.push(CommandState {
                command,
                log_path: format!(
                    "{}/{TEST_TERRAIN_NAME}/19700101000000/{}.{idx}.{TEST_TIMESTAMP_NUMERIC}.log",
                    get_terrainiumd_paths().dir_str(),
                    if is_constructor {
                        CONSTRUCTORS
                    } else {
                        DESTRUCTORS
                    }
                ),
                status: status.clone(),
            });
        });

        let mut commands = BTreeMap::new();
        commands.insert(TEST_TIMESTAMP.to_string(), command_states);

        let mut constructors = Default::default();
        let mut destructors = Default::default();

        if is_constructor {
            constructors = commands;
        } else {
            destructors = commands;
        }

        TerrainState {
            session_id: TEST_TIMESTAMP_NUMERIC.to_string(),
            terrain_name: TEST_TERRAIN_NAME.to_string(),
            biome_name: EXAMPLE_BIOME.to_string(),
            toml_path,
            terrain_dir,
            is_background: false,
            start_timestamp: "".to_string(),
            end_timestamp: "".to_string(),
            envs: expected_env_vars_example_biome(),
            constructors,
            destructors,
        }
    }
}
