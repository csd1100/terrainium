use crate::common::constants::TERRAINIUMD_TMP_DIR;
use crate::daemon::types::terrain_state::TerrainState;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::{create_dir_all, File};
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, RwLock};
use tokio::time;
use tracing::{debug, trace};

#[derive(Default, Clone, Debug)]
pub struct StateManager {
    files: Arc<RwLock<HashMap<String, Arc<Mutex<File>>>>>,
}

impl StateManager {
    pub async fn init() -> Self {
        let files = Arc::new(RwLock::new(HashMap::<String, Arc<Mutex<File>>>::new()));
        Self { files }
    }

    pub(crate) async fn create_state(&self, state: TerrainState) -> Result<()> {
        trace!(
            "creating state for {}({})",
            state.terrain_name(),
            state.session_id()
        );

        self.add_state_file(state.session_id(), state.terrain_name())
            .await?;
        let files = self.files.read().await;
        let state_file = files.get(state.session_id()).unwrap();
        let mut file = state_file.lock().await;
        file.write_all(serde_json::to_string_pretty(&state).unwrap().as_ref())
            .await
            .context(format!(
                "failed to write state to file: {}({})",
                state.terrain_name(),
                state.session_id()
            ))?;
        trace!("created state for {}", state.terrain_name());
        Ok(())
    }

    async fn add_state_file(&self, session_id: &str, terrain_name: &str) -> Result<()> {
        let terrain_state_dir = format!("{TERRAINIUMD_TMP_DIR}/{terrain_name}/{session_id}");
        create_dir_all(&terrain_state_dir).await?;
        trace!("creating state in {terrain_state_dir}",);

        let mut files = self.files.write().await;
        let state_file = File::options()
            .create(true)
            .write(true)
            .open(format!("{terrain_state_dir}/state.json",))
            .await
            .context(format!(
                "failed to open state file for {terrain_name}({session_id})",
            ))?;
        files.insert(session_id.to_string(), Arc::new(Mutex::new(state_file)));
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

    async fn cleanup(files_map: Arc<RwLock<HashMap<String, Arc<Mutex<File>>>>>) {
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
