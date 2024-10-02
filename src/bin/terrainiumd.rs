use anyhow::Result;
use std::path::PathBuf;
use terrainium::common::constants::{TERRAINIUMD_SOCKET, TERRAINIUMD_TMP_DIR};
use terrainium::daemon::handlers::handle_request;
use terrainium::daemon::types::context::Context;
use terrainium::daemon::types::daemon::Daemon;
use terrainium::daemon::types::daemon_socket::DaemonSocket;
use tokio::signal::unix::{signal, SignalKind};
use tokio_stream::StreamExt;
use tracing::{event, instrument, subscriber, Level};
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

    subscriber::set_global_default(subscriber).expect("unable to set global subscriber");

    let context = Context::new();

    let mut daemon = Daemon::new(PathBuf::from(TERRAINIUMD_SOCKET))
        .await
        .expect("to create new terrainium daemon");

    let mut sigterm = signal(SignalKind::terminate()).expect("to create a SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("to create a SIGTERM handler");

    tokio::select! {

        _ = tokio::signal::ctrl_c() => {
            event!(Level::INFO, "Received signal Ctr+c, Shutting down");
            context.cancel();
        }

        _ = sigterm.recv() => {
            event!(Level::INFO, "Received signal SIGTERM, Shutting down");
            context.cancel();
        }

        _ = sigint.recv() => {
            event!(Level::INFO, "Received signal SIGINT, Shutting down");
            context.cancel();
        }

        result = start_listener(&context, &mut daemon) => {
            if let Err(e) = result {
                event!(Level::ERROR, "Shutting down with error: {}", e.to_string());
            } else {
                event!(Level::INFO, "Shutting down");
            }
        }
    }

    Ok(())
}

async fn start_listener(context: &Context, daemon: &mut Daemon) -> Result<()> {
    let mut futures = Vec::new();

    let listener = daemon.listener();
    while let Some(socket) = listener.next().await.transpose()? {
        event!(Level::TRACE, "received socket connection");

        if context.token().is_cancelled() {
            break;
        }

        let context = context.clone();
        futures.push(tokio::spawn(async move {
            handle_request(context, DaemonSocket::new(socket)).await;
        }));
    }

    futures.iter().for_each(|f| f.abort());

    Ok(())
}
