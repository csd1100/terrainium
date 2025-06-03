use crate::common::types::terrain_state::{CommandState, CommandStatus, TerrainState};
use crate::daemon::types::state_file::StateFile;
use anyhow::{bail, Context, Result};
use std::path::Path;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct State {
    state: TerrainState,
    file: Mutex<StateFile>,
}

impl State {
    pub async fn new(state: TerrainState) -> Result<Self> {
        let mut file = StateFile::create(&state.state_file())
            .await
            .context("failed to create state file")?;
        file.write_state(&state)
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
        timestamp: &str,
        is_constructor: bool,
        commands: Vec<CommandState>,
    ) -> Result<()> {
        self.state
            .add_commands_if_necessary(timestamp, is_constructor, commands);
        let file = &mut self.file.lock().await;
        file.write_state(&self.state)
            .await
            .context("failed to update state in the file")
    }

    pub async fn update_command_status(
        &mut self,
        is_constructor: bool,
        timestamp: &str,
        index: usize,
        status: CommandStatus,
    ) -> Result<()> {
        self.state
            .update_command_status(is_constructor, timestamp, index, status)
            .context("failed to update status")?;
        let file = &mut self.file.lock().await;
        file.write_state(&self.state)
            .await
            .context("failed to update state in the file")
    }

    pub async fn update_end_timestamp(&mut self, timestamp: String) -> Result<()> {
        self.state.update_end_timestamp(timestamp);
        let file = &mut self.file.lock().await;
        file.write_state(&self.state)
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
