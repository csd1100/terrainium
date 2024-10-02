use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use terrainium::common::constants::TERRAINIUMD_SOCKET;
use terrainium::daemon::args::DaemonArgs;
use terrainium::daemon::handlers::handle_request;
use terrainium::daemon::logging::init_logging;
use terrainium::daemon::types::daemon::Daemon;
use terrainium::daemon::types::daemon_socket::DaemonSocket;
use tokio_stream::StreamExt;
use tracing::metadata::LevelFilter;
use tracing::{event, instrument, Level};

#[instrument]
#[tokio::main]
async fn main() -> Result<()> {
    let args = DaemonArgs::parse();

    let (subscriber, (_file_guard, _out_guard)) = init_logging(LevelFilter::from(args.log_level));

    tracing::subscriber::set_global_default(subscriber).expect("unable to set global subscriber");

    let mut daemon = Daemon::new(PathBuf::from(TERRAINIUMD_SOCKET), args.force)
        .await
        .context("to create new terrainium daemon")?;

    let listener = daemon.listener();

    while let Some(socket) = listener.next().await.transpose()? {
        event!(Level::TRACE, "received socket connection");
        let _ = tokio::spawn(async move {
            handle_request(DaemonSocket::new(socket)).await;
        })
            .await;
    }

    Ok(())
}
