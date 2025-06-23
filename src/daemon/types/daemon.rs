use crate::common::constants::get_terrainiumd_socket;
use anyhow::{bail, Context, Result};
use std::fs::{create_dir_all, remove_file};
use std::path::{Path, PathBuf};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tracing::{error, info, instrument, warn};

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

        // if !self.pid_file.exists() {
        //     return Ok(false);
        // }
        //
        // let pid = std::fs::read_to_string(&self.pid_file)
        //     .context("failed to read terrainiumd pid file")?;
        //
        // let is_running = Command::new(
        //     "kill".to_string(),
        //     vec!["-0".to_string(), pid],
        //     Some(std::env::temp_dir()),
        // );
        //
        // let running = self
        //     .executor
        //     .wait(None, is_running, true)
        //     .context("failed to check if service is running")?;

        if !path.exists() {
            warn!("creating directories required for path: {path:?}");
            create_dir_all(path.parent().expect("socket to have parent"))
                .expect("creating parent directory");
        } else if path.exists() && force {
            warn!("cleaning up daemon socket on path: {path:?}");
            remove_file(&path).expect("dangling socket to be cleaned up");
        } else {
            error!("daemon socket already exists on path: {path:?}");
            bail!("daemon socket already exists");
        }

        info!("creating daemon on path: {path:?}");
        let listener = UnixListenerStream::new(
            UnixListener::bind(get_terrainiumd_socket()).context("failed to bind to socket")?,
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
