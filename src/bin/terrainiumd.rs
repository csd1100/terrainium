use anyhow::Result;
use std::path::PathBuf;
use terrainium::common::constants::{TERRAINIUMD_SOCKET, TERRAINIUMD_TMP_DIR};
use terrainium::daemon::handlers::handle_request;
use terrainium::daemon::types::daemon::Daemon;
use terrainium::daemon::types::daemon_socket::DaemonSocket;
use tokio_stream::StreamExt;
use tracing::instrument;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{fmt, Registry};

#[instrument]
#[tokio::main]
async fn main() -> Result<()> {
    let appender = tracing_appender::rolling::daily(TERRAINIUMD_TMP_DIR, "terrainiumd.log");
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(appender);
    let (non_blocking_stdout, _guard) = tracing_appender::non_blocking(std::io::stdout());

    let subscriber = Registry::default()
        .with(
            fmt::Layer::default()
                .with_writer(non_blocking_file)
                .with_ansi(false)
                .with_target(false),
        )
        .with(fmt::Layer::default().with_writer(non_blocking_stdout));

    tracing::subscriber::set_global_default(subscriber).expect("unable to set global subscriber");

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
