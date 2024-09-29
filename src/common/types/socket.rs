use anyhow::{Context, Result};
use prost::Message;
use prost_types::Any;
use tokio::io::{AsyncReadExt, AsyncWriteExt, Interest};
use tokio::net::UnixStream;

pub trait Socket {
    fn stream(&mut self) -> &mut UnixStream;
    fn is_ready(&mut self) -> impl std::future::Future<Output = Result<bool>> + Send;
    fn read(&mut self) -> impl std::future::Future<Output = Result<Any>> + Send;
    fn write_and_stop(
        &mut self,
        payload: Any,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    fn stop_write(&mut self) -> impl std::future::Future<Output = Result<()>> + Send;
}

pub async fn socket_is_ready(socket: &mut impl Socket) -> Result<bool> {
    let ready_state = socket
        .stream()
        .ready(Interest::READABLE | Interest::WRITABLE)
        .await?;
    Ok(ready_state.is_readable() & ready_state.is_writable())
}

pub async fn socket_read(socket: &mut impl Socket) -> Result<Any> {
    socket
        .is_ready()
        .await
        .context("failed to check if stream is ready")?;

    let mut buf = Vec::<u8>::new();
    let _ = socket.stream().read_to_end(&mut buf).await?;
    Ok(Any::decode(buf.as_ref())?)
}

pub async fn socket_write_and_stop(socket: &mut impl Socket, payload: Any) -> Result<()> {
    socket
        .is_ready()
        .await
        .context("failed to check if stream is ready")?;

    socket
        .stream()
        .write_all(payload.encode_to_vec().as_ref())
        .await
        .context("failed to write to socket")?;

    socket
        .stream()
        .flush()
        .await
        .context("failed to flush the data")?;

    socket.stop_write().await?;

    println!("Executing: {:?}", payload);
    Ok(())
}

pub async fn socket_stop_write(socket: &mut impl Socket) -> Result<()> {
    socket
        .stream()
        .shutdown()
        .await
        .context("failed to shutdown socket")
}
