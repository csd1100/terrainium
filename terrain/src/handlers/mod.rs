use std::path::PathBuf;

use anyhow::{Context as _, Result, bail};
use terrainium_lib::pb;
use terrainium_lib::socket::Socket;
use tokio::net::UnixStream;

pub mod init;
pub mod status;

/// Client that will connect to Daemon
pub struct Client {
    path: PathBuf,
    stream: Option<UnixStream>,
}

#[async_trait::async_trait]
impl Socket<pb::Response, pb::Request> for Client {
    async fn connect(&mut self) -> Result<()> {
        let stream = UnixStream::connect(&self.path)
            .await
            .context("failed to connect to the daemon")?;

        self.stream = Some(stream);

        Ok(())
    }

    fn stream(&mut self) -> &mut UnixStream {
        self.stream
            .as_mut()
            .expect("expect client to be connected before calling this method")
    }
}

impl Client {
    pub fn new(path: PathBuf) -> Result<Self> {
        if !path.exists() {
            bail!("Daemon Socket does not exist at: {path:?}");
        }

        if !path.is_absolute() {
            bail!("Daemon socket path: {path:?} should be absolute");
        }

        Ok(Self { path, stream: None })
    }
}
