use crate::common::types::socket::{
    Socket, socket_is_ready, socket_read, socket_stop_write, socket_write_and_stop,
};
use anyhow::Result;
#[cfg(test)]
use mockall::mock;
use prost_types::Any;
use tokio::net::UnixStream;

#[derive(Debug)]
pub struct DaemonSocket {
    stream: UnixStream,
}

impl DaemonSocket {
    pub fn new(stream: UnixStream) -> Self {
        DaemonSocket { stream }
    }
}

impl Socket for DaemonSocket {
    fn stream(&mut self) -> &mut UnixStream {
        &mut self.stream
    }

    async fn ready(&mut self) -> Result<bool> {
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

#[cfg(test)]
mock! {
    #[derive(Debug)]
    pub DaemonSocket {
        pub async fn new(stream: UnixStream) -> Result<DaemonSocket>;
    }

    impl Socket for DaemonSocket {
        fn stream(&mut self) -> &mut UnixStream;
        async fn ready(&mut self) -> Result<bool>;
        async fn read(&mut self) -> Result<Any>;
        async fn write_and_stop(&mut self, payload: Any) -> Result<()>;
        async fn stop_write(&mut self) -> Result<()>;
    }
}
