use anyhow::Result;
use std::path::PathBuf;
use terrainium::common::constants::TERRAINIUMD_SOCKET;
use terrainium::daemon::handlers::handle_request;
use terrainium::daemon::types::daemon::Daemon;
use terrainium::daemon::types::server_socket::DaemonSocket;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let mut daemon = Daemon::new(PathBuf::from(TERRAINIUMD_SOCKET))
        .await
        .expect("to create new terrainium daemon");

    let listener = daemon.listener();

    while let Some(socket) = listener.next().await.transpose()? {
        let _ = tokio::spawn(async move {
            handle_request(DaemonSocket::new(socket)).await;
        })
        .await;
    }

    Ok(())
}
