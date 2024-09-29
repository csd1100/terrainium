use crate::common::constants::TERRAINIUMD_SOCKET;
use anyhow::{anyhow, Context, Result};
use std::fs::{create_dir_all, remove_file};
use std::path::{Path, PathBuf};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;

pub struct Daemon {
    path: PathBuf,
    listener: UnixListenerStream,
}

impl Daemon {
    pub async fn new(path: PathBuf) -> Result<Daemon> {
        if !path.is_absolute() {
            return Err(anyhow!("path for socket should be absolute"));
        }

        if path.exists() {
            remove_file(&path).expect("dangling socket to be cleaned up");
        }

        if !path.parent().expect("socket to have parent").exists() {
            create_dir_all(path.parent().expect("socket to have parent"))
                .expect("creating parent directory");
        }

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
