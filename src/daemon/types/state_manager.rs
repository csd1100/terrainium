use crate::common::constants::TERRAIN_STATE_FILE_NAME;
use crate::common::types::terrain_state::{CommandState, TerrainState};
use crate::daemon::types::history::History;
use crate::daemon::types::state::State;
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;
use tracing::{debug, trace};

pub type StoredState = Arc<RwLock<State>>;
pub type StoredHistory = Arc<RwLock<History>>;

#[derive(Default, Clone, Debug)]
pub struct StateManager {
    states: Arc<RwLock<HashMap<String, StoredState>>>,
    histories: Arc<RwLock<HashMap<String, StoredHistory>>>,
}

fn state_key(terrain_name: &str, session_id: &str) -> String {
    format!("{terrain_name}({session_id})")
}

impl StateManager {
    pub async fn init() -> Self {
        let states = Arc::new(RwLock::new(HashMap::<String, StoredState>::new()));
        let histories = Arc::new(RwLock::new(HashMap::<String, StoredHistory>::new()));
        Self { states, histories }
    }

    pub(crate) async fn get_or_create_history(&self, terrain_name: &str) -> Result<StoredHistory> {
        debug!("getting history for terrain {terrain_name}");
        let history = self.histories.read().await;
        if let Some(h) = history.get(terrain_name) {
            debug!("history already exists for terrain {terrain_name}");
            Ok(h.clone())
        } else {
            drop(history);
            debug!("creating history for terrain {terrain_name}");
            let history = Arc::new(RwLock::new(History::read(terrain_name).await?));
            let mut histories = self.histories.write().await;
            histories.insert(terrain_name.to_string(), history.clone());
            Ok(history)
        }
    }

    pub(crate) async fn create_state(&self, terrain_state: TerrainState) -> Result<()> {
        let terrain_name = terrain_state.terrain_name().to_string();
        let session_id = terrain_state.session_id().to_string();

        trace!(
            terrain_name = terrain_name,
            session_id = session_id,
            "creating state"
        );

        let history = self
            .get_or_create_history(&terrain_name)
            .await
            .context(format!("failed to create history file {terrain_name}"))?;

        let state = Arc::new(RwLock::new(
            State::new(history, terrain_state)
                .await
                .context("failed to create state")?,
        ));

        self.states
            .write()
            .await
            .insert(state_key(&terrain_name, &session_id), state.clone());

        trace!(
            terrain_name = terrain_name,
            session_id = session_id,
            "created state"
        );
        Ok(())
    }

    pub(crate) async fn add_state(
        &self,
        terrain_name: &str,
        session_id: &str,
    ) -> Result<StoredState> {
        trace!(
            terrain_name = terrain_name,
            session_id = session_id,
            "adding state to manager"
        );

        let state_file =
            TerrainState::get_state_dir(terrain_name, session_id).join(TERRAIN_STATE_FILE_NAME);

        let state = Arc::new(RwLock::new(
            State::read(&state_file)
                .await
                .context("failed to create state")?,
        ));

        self.states
            .write()
            .await
            .insert(state_key(terrain_name, session_id), state.clone());

        trace!(
            terrain_name = terrain_name,
            session_id = session_id,
            "created state"
        );
        Ok(state)
    }

    pub(crate) async fn add_commands_if_necessary(
        &self,
        terrain_name: &str,
        session_id: &str,
        timestamp: &str,
        is_constructor: bool,
        commands: Vec<CommandState>,
    ) -> Result<()> {
        let stored_state = self.refreshed_state(terrain_name, session_id).await?;
        let mut state = stored_state.write().await;

        debug!(
            terrain_name = terrain_name,
            session_id = session_id,
            timestamp = timestamp,
            "adding commands"
        );
        let history = self
            .get_or_create_history(terrain_name)
            .await
            .context(format!("failed to create history file {terrain_name}"))?;

        state
            .add_commands_if_necessary(history, timestamp, is_constructor, commands)
            .await
            .context("failed to add commands")
    }

    pub(crate) async fn update_end_time(
        &self,
        terrain_name: &str,
        session_id: &str,
        end_timestamp: String,
    ) -> Result<()> {
        let stored_state = self.refreshed_state(terrain_name, session_id).await?;
        let mut state = stored_state.write().await;

        debug!(
            terrain_name = terrain_name,
            session_id = session_id,
            "updating end_timestamp"
        );
        let history = self
            .get_or_create_history(terrain_name)
            .await
            .context(format!("failed to create history file {terrain_name}"))?;
        state
            .update_end_timestamp(history, end_timestamp)
            .await
            .context("failed to update end_timestamp")
    }

    pub(crate) async fn refreshed_state(
        &self,
        terrain_name: &str,
        session_id: &str,
    ) -> Result<StoredState> {
        let states = self.states.read().await;
        if let Some(state) = states.get(&state_key(terrain_name, session_id)) {
            debug!(
                terrain_name = terrain_name,
                session_id = session_id,
                "state already exists"
            );
            Ok(state.clone())
        } else {
            drop(states);

            let state_file =
                TerrainState::get_state_dir(terrain_name, session_id).join(TERRAIN_STATE_FILE_NAME);

            if state_file.exists() {
                debug!(
                    terrain_name = terrain_name,
                    session_id = session_id,
                    "refreshing state"
                );

                self.add_state(terrain_name, session_id).await
            } else {
                bail!("state file {} doesn't exist", state_file.display());
            }
        }
    }

    pub fn setup_cleanup(&self) {
        trace!("setting up state cleanup timer");
        let states = self.states.clone();
        tokio::task::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(180));
            loop {
                interval.tick().await;
                Self::cleanup(states.clone()).await;
            }
        });
    }

    async fn cleanup(files_map: Arc<RwLock<HashMap<String, StoredState>>>) {
        trace!("cleaning up state files");
        let mut cleanups = Vec::new();
        let mut map = files_map.write().await;
        map.iter().for_each(|(name, file)| {
            if file.try_read().is_ok() && file.try_write().is_ok() {
                cleanups.push(name.clone());
            }
        });
        cleanups.into_iter().for_each(|name| {
            debug!("cleaning up state file for {name}");
            map.remove(&name);
        });
    }
}
