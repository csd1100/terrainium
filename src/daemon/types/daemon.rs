use crate::common::constants::TERRAINIUMD_SOCKET;
use anyhow::{bail, Context, Result};
use std::fs::{create_dir_all, remove_file};
use std::path::{Path, PathBuf};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tracing::{event, instrument, Level};

pub struct Daemon {
    path: PathBuf,
    listener: UnixListenerStream,
}

impl Daemon {
    #[instrument]
    pub async fn new(path: PathBuf, force: bool) -> Result<Daemon> {
        if !path.is_absolute() {
            bail!("path for socket should be absolute");
        }

        if !path.exists() {
            event!(
                Level::WARN,
                "creating directories required for path: {path:?}"
            );
            create_dir_all(path.parent().expect("socket to have parent"))
                .expect("creating parent directory");
        } else if path.exists() && force {
            event!(Level::WARN, "cleaning up daemon socket on path: {path:?}");
            remove_file(&path).expect("dangling socket to be cleaned up");
        } else {
            event!(
                Level::ERROR,
                "daemon socket already exists on path: {path:?}"
            );
            bail!("daemon socket already exists");
        }

        event!(Level::INFO, "creating daemon on path: {path:?}");
        let listener = UnixListenerStream::new(
            UnixListener::bind(TERRAINIUMD_SOCKET).context("failed to bind to socket")?,
        );

        Ok(Daemon { path, listener })
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub fn listener(&mut self) -> &mut UnixListenerStream {
        &mut self.listener
    }
}
