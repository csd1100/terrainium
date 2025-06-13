use crate::common::constants::TERRAIN_HISTORY_FILE_NAME;
use crate::common::types::pb::status_request::Identifier;
use crate::common::utils;
use anyhow::{bail, Result};
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::sync::Mutex;
use tracing::{debug, instrument, trace};

#[derive(Debug)]
pub struct History {
    history: Vec<String>,
    file: Mutex<HistoryFile>,
}

impl History {
    pub fn get_path(state_directory: &str, terrain_name: &str) -> PathBuf {
        PathBuf::from(&format!(
            "{state_directory}/{terrain_name}/{TERRAIN_HISTORY_FILE_NAME}"
        ))
    }

    pub(crate) async fn read(
        state_directory: &str,
        terrain_name: &str,
        size: usize,
    ) -> Result<Self> {
        let mut file =
            HistoryFile::create(&Self::get_path(state_directory, terrain_name), size).await?;
        let history = file.read().await?;
        let file = Mutex::new(file);
        Ok(Self { history, file })
    }

    #[instrument(skip(self))]
    pub(crate) async fn add(&mut self, session_id: String) -> Result<()> {
        trace!("adding session");
        if self.history[0] == session_id {
            debug!("session already exists");
            Ok(())
        } else {
            let hist = self.history.as_mut_slice();
            hist.rotate_right(1);
            hist[0] = session_id;
            let mut file = self.file.lock().await;
            file.write(&self.history).await
        }
    }

    #[instrument(skip(self))]
    pub(crate) fn get_session(&self, identifier: Identifier) -> Result<String> {
        debug!("getting session information from request");
        match identifier {
            Identifier::SessionId(session_id) => Ok(session_id),
            Identifier::Recent(recent) => {
                if self.history.len() < recent as usize {
                    bail!(
                        "invalid request for {recent}th entry, history size is only {}",
                        self.history.len()
                    );
                }

                let session_id = self.history[recent as usize].clone();
                if session_id.is_empty() {
                    bail!("no session id found at index {recent}")
                }
                Ok(session_id)
            }
        }
    }
}

#[derive(Debug)]
struct HistoryFile {
    file: File,
    size: usize,
}

impl HistoryFile {
    #[instrument]
    async fn create(path: &Path, size: usize) -> Result<Self> {
        trace!("creating history file");
        Ok(Self {
            file: utils::create_file(path).await?,
            size,
        })
    }

    async fn write(&mut self, history: &[String]) -> Result<()> {
        assert!(history.len() <= self.size);
        utils::write_to_file(&mut self.file, history.join("\n")).await
    }

    async fn read(&mut self) -> Result<Vec<String>> {
        let data = utils::read_from_file(&mut self.file).await?;
        let lines: Vec<String> = data.lines().map(|line| line.to_string()).collect();
        assert!(lines.len() <= self.size);
        let mut history = vec!["".to_string(); self.size];
        lines.into_iter().enumerate().for_each(|(index, line)| {
            history[index] = line;
        });
        Ok(history)
    }
}
