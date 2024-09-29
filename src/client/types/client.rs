use crate::common::types::socket::{
    socket_is_ready, socket_read, socket_stop_write, socket_write_and_stop, Socket,
};
use anyhow::Result;
use anyhow::{anyhow, Context as AnyhowContext};
use mockall::mock;
use prost_types::Any;
use std::path::PathBuf;
use tokio::net::UnixStream;

#[derive(Debug)]
pub struct Client {
    stream: UnixStream,
}

impl Socket for Client {
    fn stream(&mut self) -> &mut UnixStream {
        &mut self.stream
    }

    async fn is_ready(&mut self) -> Result<bool> {
        socket_is_ready(self).await
    }

    async fn read(&mut self) -> Result<Any> {
        socket_read(self).await
    }

    async fn write_and_stop(&mut self, payload: Any) -> Result<()> {
        socket_write_and_stop(self, payload).await
    }

    async fn stop_write(&mut self) -> Result<()> {
        socket_stop_write(self).await
    }
}

impl Client {
    pub async fn new(path: PathBuf) -> Result<Client> {
        if !path.exists() {
            return Err(anyhow!("Daemon Socket does not exist: {}", path.display()));
        }

        if !path.is_absolute() {
            return Err(anyhow!(
                "Daemon path: {} should be absolute",
                path.display()
            ));
        }

        let stream = UnixStream::connect(path.clone())
            .await
            .context("failed to connect to the daemon")?;

        Ok(Client { stream })
    }
}

mock! {
    #[derive(Debug)]
    pub Client {
        pub async fn new(path: PathBuf) -> Result<Client>;
    }

    impl Socket for Client {
        fn stream(&mut self) -> &mut UnixStream;
        async fn is_ready(&mut self) -> Result<bool>;
        async fn read(&mut self) -> Result<Any>;
        async fn write_and_stop(&mut self, payload: Any) -> Result<()>;
        async fn stop_write(&mut self) -> Result<()>;
    }
}
