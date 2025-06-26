use std::path::PathBuf;

use anyhow::{Context as AnyhowContext, Result, bail};
#[cfg(test)]
use mockall::mock;
use prost_types::Any;
use tokio::net::UnixStream;

use crate::client::types::proto::{ProtoRequest, ProtoResponse};
use crate::common::types::pb;
use crate::common::types::pb::response::Payload;
use crate::common::types::socket::{
    Socket, socket_is_ready, socket_read, socket_stop_write, socket_write_and_stop,
};

#[derive(Debug)]
pub struct Client {
    stream: UnixStream,
}

impl Socket for Client {
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

impl Client {
    pub async fn new(path: PathBuf) -> Result<Client> {
        if !path.exists() {
            bail!("Daemon Socket does not exist: {path:?}");
        }

        if !path.is_absolute() {
            bail!("Daemon path: {path:?} should be absolute");
        }

        let stream = UnixStream::connect(path)
            .await
            .context("failed to connect to the daemon")?;

        Ok(Client { stream })
    }

    pub async fn request(&mut self, payload: ProtoRequest) -> Result<ProtoResponse> {
        let request: Any = match &payload {
            ProtoRequest::Activate(activate) => Any::from_msg(activate),
            ProtoRequest::Deactivate(deactivate) => Any::from_msg(deactivate),
            ProtoRequest::Execute(commands) => Any::from_msg(commands),
            ProtoRequest::Status(status) => Any::from_msg(status),
        }
        .context(format!("failed to convert request {:?} to any", payload))?;

        self.write_and_stop(request).await?;

        let response: Any = self.read().await?;
        let response: pb::Response = response
            .to_msg()
            .context("failed to parse activate response")?;

        if let Some(payload) = response.payload {
            match payload {
                Payload::Error(err) => {
                    bail!("error response from daemon: {err}");
                }
                Payload::Body(body) => match body.message {
                    None => Ok(ProtoResponse::Success),
                    Some(status) => Ok(ProtoResponse::Status(Box::new(status))),
                },
            }
        } else {
            bail!("no response payload for request {:?}", payload);
        }
    }
}

#[cfg(test)]
mock! {
    #[derive(Debug)]
    pub Client {
        pub async fn new(path: PathBuf) -> Result<Self>;
        pub async fn request(&mut self, payload: ProtoRequest) -> Result<ProtoResponse>;
    }

    impl Socket for Client {
        fn stream(&mut self) -> &mut UnixStream;
        async fn ready(&mut self) -> Result<bool>;
        async fn read(&mut self) -> Result<Any>;
        async fn write_and_stop(&mut self, payload: Any) -> Result<()>;
        async fn stop_write(&mut self) -> Result<()>;
    }
}
