use anyhow::{Context as _, Result};
#[cfg(any(test, feature = "test-exports"))]
use mockall::automock;
use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt, Interest};
use tokio::net::UnixStream;

/// Communicate using protobuf over Unix Sockets
///
/// For Client: [Out] should be [pb::Request] and [In] should be [pb::Response]
/// For Daemon: [Out] should be [pb::Response] and [In] should be [pb::Request]
#[cfg_attr(any(test, feature = "test-exports"), automock)]
#[async_trait::async_trait]
pub trait Socket<In: Message + Default, Out: Message> {
    /// Connect with socket
    ///
    /// Must be called by [Client]
    /// Must NOT be called by [DaemonSocket]
    ///
    /// # Panics
    ///
    /// When called by [DaemonSocket]
    async fn connect(&mut self) -> Result<()>;

    /// Fetch stream to communicate over
    fn stream(&mut self) -> &mut UnixStream;

    /// Waits for socket to be readable and writable
    async fn ready(&mut self) -> Result<bool> {
        let ready_state = self
            .stream()
            .ready(Interest::READABLE | Interest::WRITABLE)
            .await
            .context("failed to wait for socket to be readable and writable")?;
        Ok(ready_state.is_readable() & ready_state.is_writable())
    }

    /// Reads message over the socket
    async fn read(&mut self) -> Result<In> {
        self.ready()
            .await
            .context("failed to check if stream is ready")?;

        let mut length_bytes = [0u8; 4];
        self.stream()
            .read_exact(&mut length_bytes)
            .await
            .context("failed to read length of the message")?;

        let length = u32::from_be_bytes(length_bytes) as usize;

        // Read the message
        let mut buffer = vec![0u8; length];
        self.stream()
            .read_exact(&mut buffer)
            .await
            .context("failed to read the message")?;

        In::decode(&buffer[..]).context("failed to decode the message read from socket")
    }

    /// Writes the data to the socket
    async fn write(&mut self, payload: &Out) -> Result<()> {
        let message = payload.encode_to_vec();
        let length = (message.len() as u32).to_be_bytes();
        self.stream()
            .write_all(&length)
            .await
            .context("failed to write length to socket")?;

        self.stream()
            .write_all(&message)
            .await
            .context("failed to write message to socket")?;

        self.stream()
            .flush()
            .await
            .context("failed to flush the socket")?;

        Ok(())
    }

    /// Shuts down the Socket
    async fn shutdown(&mut self) -> Result<()> {
        self.stream()
            .shutdown()
            .await
            .context("failed to shutdown the socket")
    }

    /// Connects, Writes the [Out], Reads the [In], and Shutdowns
    ///
    /// Helper for client to communicate with Daemon
    async fn request(&mut self, req: &Out) -> Result<In> {
        self.connect()
            .await
            .context("failed to connect to daemon")?;

        self.write(req)
            .await
            .context("failed to write request to daemon")?;

        let response = self
            .read()
            .await
            .context("failed to read response from daemon")?;

        self.shutdown()
            .await
            .context("failed to close the connection")?;

        Ok(response)
    }
}
