use crate::common::constants::TERRAINIUMD_TMP_DIR;
use crate::common::types::pb;
use crate::common::types::pb::status_request::{HistoryArg, Identifier};
use crate::common::utils;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::sync::Mutex;
use tracing::debug;

#[derive(Debug)]
pub struct History {
    history: [String; 3],
    file: Mutex<HistoryFile>,
}

impl History {
    pub fn get_path(terrain_name: &str) -> PathBuf {
        PathBuf::from(&format!("{TERRAINIUMD_TMP_DIR}/{terrain_name}/history"))
    }

    pub(crate) async fn read(terrain_name: &str) -> Result<Self> {
        let mut file = HistoryFile::create(&Self::get_path(terrain_name)).await?;
        let history = file.read().await?;
        let file = Mutex::new(file);
        Ok(Self { history, file })
    }

    pub(crate) async fn add(&mut self, session_id: String) -> Result<()> {
        debug!("adding session {session_id}");
        if self.history[0] == session_id {
            debug!("session {session_id} already exists");
            Ok(())
        } else {
            let hist = self.history.as_mut_slice();
            hist.rotate_right(1);
            hist[0] = session_id;
            let mut file = self.file.lock().await;
            file.write(&self.history).await
        }
    }
    pub(crate) fn get_session(&self, identifier: Identifier) -> Result<String> {
        match identifier {
            Identifier::SessionId(session_id) => Ok(session_id),
            Identifier::History(history) => {
                let history = pb::status_request::HistoryArg::try_from(history)
                    .context("failed to parse history")?;
                match history {
                    HistoryArg::HistoryUnspecified => {
                        bail!("invalid history arg unspecified");
                    }
                    HistoryArg::HistoryRecent => Ok(self.history[0].clone()),
                    HistoryArg::HistoryRecent1 => Ok(self.history[1].clone()),
                    HistoryArg::HistoryRecent2 => Ok(self.history[2].clone()),
                    HistoryArg::HistoryCurrent => {
                        bail!("unexpected history arg current");
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
struct HistoryFile {
    file: File,
}

impl HistoryFile {
    async fn create(path: &Path) -> Result<Self> {
        Ok(Self {
            file: utils::create_file(path).await?,
        })
    }

    async fn write(&mut self, history: &[String; 3]) -> Result<()> {
        utils::write_to_file(&mut self.file, history.join("\n")).await
    }

    async fn read(&mut self) -> Result<[String; 3]> {
        let data = utils::read_from_file(&mut self.file).await?;
        let lines: Vec<String> = data.lines().map(|line| line.to_string()).collect();
        assert!(lines.len() <= 3);
        let mut history: [String; 3] = Default::default();
        lines.into_iter().enumerate().for_each(|(index, line)| {
            history[index] = line;
        });
        Ok(history)
    }
}
