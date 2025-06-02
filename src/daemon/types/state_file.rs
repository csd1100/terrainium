use crate::daemon::types::terrain_state::TerrainState;
use anyhow::{anyhow, Context, Result};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

#[derive(Debug)]
pub(crate) struct StateFile {
    file: File,
}

impl StateFile {
    pub(crate) async fn new(path: &Path) -> Result<Self> {
        let file = File::options()
            .create(true)
            .read(true)
            .write(true)
            .append(false)
            .truncate(true)
            .open(path)
            .await
            .context(format!("failed to open state file for {path:?})"))?;
        Ok(Self { file })
    }

    pub(crate) async fn write_state(&mut self, state: &TerrainState) -> Result<()> {
        self.file
            .rewind()
            .await
            .context("failed to rewind state file")?;

        self.file
            .set_len(0)
            .await
            .context("failed to truncate file handle")?;

        let json =
            serde_json::to_string_pretty(state).context("failed to serialize terrain state")?;
        self.file
            .write_all(json.as_bytes())
            .await
            .context("failed to write terrain state")?;
        self.file.flush().await?;

        self.file
            .rewind()
            .await
            .context("failed to rewind file handle")?;
        Ok(())
    }

    pub(crate) async fn read_state(&mut self) -> Result<TerrainState> {
        self.file
            .rewind()
            .await
            .context("failed to rewind file handle")?;

        let mut buf = String::new();
        self.file
            .read_to_string(&mut buf)
            .await
            .context("failed to read terrain state")?;

        self.file
            .rewind()
            .await
            .context("failed to rewind file handle")?;

        serde_json::from_str(&buf).context(anyhow!("failed to deserialize terrain state"))
    }
}
