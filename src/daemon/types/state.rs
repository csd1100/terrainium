use crate::common::types::terrain_state::{CommandState, CommandStatus, TerrainState};
use crate::common::utils;
use crate::daemon::types::state_manager::StoredHistory;
use anyhow::{bail, Context, Result};
use std::path::Path;
use tokio::fs::File;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

#[derive(Debug)]
pub struct State {
    state: TerrainState,
    file: Mutex<StateFile>,
}

impl State {
    pub async fn new(history: StoredHistory, state: TerrainState) -> Result<Self> {
        debug!(
            terrain_name = state.terrain_name(),
            session_id = state.session_id(),
            "creating new state",
        );
        let mut file = StateFile::create(&state.state_file())
            .await
            .context("failed to create state file")?;
        file.write_state(history, &state)
            .await
            .context("failed to write initial state")?;
        let file = Mutex::new(file);
        Ok(Self { state, file })
    }

    pub async fn read(path: &Path) -> Result<Self> {
        if !path.exists() {
            bail!("state file {path:?} does not exist");
        }
        let mut file = StateFile::create(path)
            .await
            .context("failed to read state file")?;
        let state = file
            .read_state()
            .await
            .context("failed to read state from the file")?;
        let file = Mutex::new(file);
        Ok(Self { state, file })
    }

    pub async fn add_commands_if_necessary(
        &mut self,
        history: StoredHistory,
        timestamp: &str,
        is_constructor: bool,
        commands: Vec<CommandState>,
    ) -> Result<()> {
        self.state
            .add_commands_if_necessary(timestamp, is_constructor, commands);
        let file = &mut self.file.lock().await;
        file.write_state(history, &self.state)
            .await
            .context("failed to update state in the file")
    }

    #[instrument(skip(self, history))]
    pub async fn update_command_status(
        &mut self,
        history: StoredHistory,
        is_constructor: bool,
        timestamp: &str,
        index: usize,
        status: CommandStatus,
    ) -> Result<()> {
        debug!(
            terrain_name = self.state.terrain_name(),
            session_id = self.state.session_id(),
            "updating command status"
        );
        self.state
            .update_command_status(is_constructor, timestamp, index, status)
            .context("failed to update status")?;
        let file = &mut self.file.lock().await;
        file.write_state(history, &self.state)
            .await
            .context("failed to update state in the file")
    }

    pub async fn update_end_timestamp(
        &mut self,
        history: StoredHistory,
        timestamp: String,
    ) -> Result<()> {
        self.state.update_end_timestamp(timestamp);
        let file = &mut self.file.lock().await;
        file.write_state(history, &self.state)
            .await
            .context("failed to update state in the file")
    }

    pub fn terrain_name(&self) -> &str {
        self.state.terrain_name()
    }

    pub fn session_id(&self) -> &str {
        self.state.session_id()
    }

    pub fn commands(&self, is_constructor: bool, timestamp: &str) -> Result<Vec<CommandState>> {
        if is_constructor {
            self.state.get_constructors(timestamp)
        } else {
            self.state.get_destructors(timestamp)
        }
    }

    pub fn state(&self) -> TerrainState {
        self.state.clone()
    }
}

#[derive(Debug)]
struct StateFile {
    file: File,
}

impl StateFile {
    #[instrument]
    async fn create(path: &Path) -> Result<Self> {
        debug!("creating state file");
        Ok(Self {
            file: utils::create_file(path).await?,
        })
    }

    async fn write_state(&mut self, history: StoredHistory, state: &TerrainState) -> Result<()> {
        debug!(
            terrain_name = state.terrain_name(),
            session_id = state.session_id(),
            "writing state",
        );
        let json =
            serde_json::to_string_pretty(state).context("failed to serialize terrain state")?;
        utils::write_to_file(&mut self.file, json)
            .await
            .context("failed to state to the file")?;
        let mut history = history.write().await;
        history.add(state.session_id().to_string()).await
    }

    async fn read_state(&mut self) -> Result<TerrainState> {
        serde_json::from_str(utils::read_from_file(&mut self.file).await?.as_str())
            .context("failed to deserialize terrain state")
    }
}
