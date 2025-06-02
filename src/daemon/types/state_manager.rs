use crate::common::constants::{TERRAINIUMD_TMP_DIR, TERRAIN_STATE_FILE_NAME};
use crate::daemon::types::state_file::StateFile;
use crate::daemon::types::terrain_state::{CommandState, CommandStatus, TerrainState};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::create_dir_all;
use tokio::sync::{Mutex, RwLock};
use tokio::time;
use tracing::{debug, trace};

#[derive(Default, Clone, Debug)]
pub struct StateManager {
    files: Arc<RwLock<HashMap<String, Arc<Mutex<StateFile>>>>>,
}

pub fn get_state_paths(terrain_name: &str, session_id: &str) -> (PathBuf, PathBuf) {
    let state_dir = PathBuf::from(format!(
        "{TERRAINIUMD_TMP_DIR}/{}/{}",
        terrain_name, session_id
    ));
    let state_file = state_dir.join(TERRAIN_STATE_FILE_NAME);

    (state_dir, state_file)
}

impl StateManager {
    pub async fn init() -> Self {
        let files = Arc::new(RwLock::new(HashMap::<String, Arc<Mutex<StateFile>>>::new()));
        Self { files }
    }

    pub(crate) async fn create_state(&self, state: &TerrainState) -> Result<()> {
        trace!(
            terrain_name = state.terrain_name(),
            session_id = state.session_id(),
            "creating state"
        );

        self.add_state_file(state.terrain_name(), state.session_id(), state.state_dir())
            .await?;

        let files = self.files.read().await;
        let mut state_file = files.get(state.session_id()).unwrap().lock().await;

        state_file
            .write_state(state)
            .await
            .context("failed to write state to the file")?;

        trace!(
            terrain_name = state.terrain_name(),
            session_id = state.session_id(),
            "created state"
        );
        Ok(())
    }

    async fn add_state_file(
        &self,
        terrain_name: &str,
        session_id: &str,
        state_dir: PathBuf,
    ) -> Result<()> {
        create_dir_all(state_dir.as_path()).await?;
        trace!(
            terrain_name = terrain_name,
            session_id = session_id,
            "adding state file {state_dir:?} to state manager"
        );

        let mut files = self.files.write().await;

        files.insert(
            session_id.to_string(),
            Arc::new(Mutex::new(
                StateFile::new(&state_dir.join(TERRAIN_STATE_FILE_NAME)).await?,
            )),
        );

        Ok(())
    }

    pub(crate) async fn add_commands_if_necessary(
        &self,
        terrain_name: &str,
        session_id: &str,
        timestamp: &str,
        is_constructor: bool,
        commands: Vec<CommandState>,
    ) -> Result<()> {
        let mut state = self.fetch_state(terrain_name, session_id).await?;
        state.add_commands_if_necessary(timestamp, is_constructor, commands);

        let files = self.files.read().await;
        let mut state_file = files.get(state.session_id()).unwrap().lock().await;

        state_file
            .write_state(&state)
            .await
            .context("failed to write state to the file")?;

        Ok(())
    }

    pub(crate) async fn fetch_state(
        &self,
        terrain_name: &str,
        session_id: &str,
    ) -> Result<TerrainState> {
        self.refresh_state(terrain_name, session_id).await?;
        let files = self.files.read().await;
        let mut file = files.get(session_id).unwrap().lock().await;
        file.read_state().await
    }

    pub(crate) async fn refresh_state(&self, terrain_name: &str, session_id: &str) -> Result<()> {
        let files = self.files.read().await;
        if files.get(session_id).is_none() {
            drop(files);

            let (state_dir, state_file) = get_state_paths(terrain_name, session_id);
            if state_file.exists() {
                debug!(
                    terrain_name = terrain_name,
                    session_id = session_id,
                    "refreshing state"
                );

                self.add_state_file(terrain_name, session_id, state_dir)
                    .await?
            } else {
                bail!("state file {} doesn't exist", state_file.display());
            }
        } else {
            debug!(
                terrain_name = terrain_name,
                session_id = session_id,
                "state file already exists"
            );
        }
        Ok(())
    }

    pub(crate) async fn update_command_status(
        &self,
        terrain_name: &str,
        session_id: &str,
        timestamp: &str,
        index: usize,
        is_constructor: bool,
        status: CommandStatus,
    ) -> Result<()> {
        trace!(
            terrain_name = terrain_name,
            session_id = session_id,
            timestamp = timestamp,
            index = index,
            is_constructor = is_constructor,
            "acquiring read lock state files"
        );

        let files = self.files.read().await;

        trace!(
            terrain_name = terrain_name,
            session_id = session_id,
            timestamp = timestamp,
            index = index,
            is_constructor = is_constructor,
            "fetching state file"
        );
        let state_file = files.get(session_id).context(format!(
            "state file does not exist in state manager for session: {terrain_name}({session_id})"
        ))?;
        let mut file = state_file.lock().await;

        trace!(
            terrain_name = terrain_name,
            session_id = session_id,
            "reading and parsing state from file"
        );

        let mut state: TerrainState = file
            .read_state()
            .await
            .context("failed to read state from the file")?;

        state.update_command_status(is_constructor, timestamp, index, status)?;

        trace!(
            terrain_name = terrain_name,
            session_id = session_id,
            "writing updated state to file"
        );

        file.write_state(&state)
            .await
            .context("failed to write the state to file")?;
        Ok(())
    }

    pub fn setup_cleanup(&self) {
        let files_map = self.files.clone();
        tokio::task::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(180));
            loop {
                interval.tick().await;
                Self::cleanup(files_map.clone()).await;
            }
        });
    }

    async fn cleanup(files_map: Arc<RwLock<HashMap<String, Arc<Mutex<StateFile>>>>>) {
        trace!("cleaning up state files");
        let mut cleanups = Vec::new();
        let mut map = files_map.write().await;
        map.iter().for_each(|(name, file)| {
            if file.try_lock().is_ok() {
                cleanups.push(name.clone());
            }
        });
        cleanups.into_iter().for_each(|name| {
            debug!("cleaning up state file for {}", name);
            map.remove(&name);
        });
    }
}
