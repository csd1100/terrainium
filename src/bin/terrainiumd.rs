use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use terrainium::common::constants::{TERRAINIUMD_SOCKET, TERRAINIUMD_TMP_DIR};
use terrainium::common::execute::{CommandToRun, Execute};
use terrainium::daemon::args::DaemonArgs;
use terrainium::daemon::handlers::handle_request;
use terrainium::daemon::logging::init_logging;
use terrainium::daemon::types::config::DaemonConfig;
use terrainium::daemon::types::context::DaemonContext;
use terrainium::daemon::types::daemon::Daemon;
use terrainium::daemon::types::daemon_socket::DaemonSocket;
use tokio_stream::StreamExt;
use tracing::metadata::LevelFilter;
use tracing::{event, instrument, Level};

fn get_daemon_config() -> DaemonConfig {
    let config = DaemonConfig::from_file().unwrap_or_default();
    event!(Level::INFO, "config: {:?}", config);
    config
}

fn is_user_root() -> bool {
    let user = CommandToRun::new(
        "whoami".to_string(),
        vec![],
        None,
        &PathBuf::from(TERRAINIUMD_TMP_DIR),
    )
    .get_output();

    if let Ok(user) = user {
        let user = String::from_utf8_lossy(&user.stdout);
        event!(Level::INFO, "running service as user: {}", user.trim());
        if user.contains("root") {
            event!(
                Level::WARN,
                "running service as root is not advised, see terrainium docs for more information."
            );
            return true;
        }
    } else {
        event!(
            Level::INFO,
            "could not figure out user running service, running in non-root mode."
        );
    }
    false
}

async fn get_daemon_context() -> DaemonContext {
    let context = DaemonContext::new(is_user_root(), get_daemon_config().is_root_allowed()).await;
    context.setup_state_manager();
    context
}

#[instrument]
#[tokio::main]
async fn main() -> Result<()> {
    let args = DaemonArgs::parse();

    let (subscriber, (_file_guard, _out_guard)) = init_logging(LevelFilter::from(args.log_level));
    tracing::subscriber::set_global_default(subscriber).expect("unable to set global subscriber");

    if args.create_config {
        let res = DaemonConfig::create_file();
        if let Err(err) = res {
            event!(
                Level::ERROR,
                "failed to create terrainiumd config file: {err}",
            );
        }
        return Ok(());
    }

    let context = get_daemon_context().await;

    if context.should_exit_early() {
        event!(
            Level::ERROR,
            context = ?context,
            "exiting as service was started as root without being configured.",
        );
        anyhow::bail!("exiting as service was running as root");
    }

    let mut daemon = Daemon::new(PathBuf::from(TERRAINIUMD_SOCKET), args.force)
        .await
        .context("to create new terrainium daemon")?;
    let listener = daemon.listener();

    while let Some(socket) = listener.next().await.transpose()? {
        let context = context.clone();
        event!(Level::TRACE, "received socket connection");
        let _ = tokio::spawn(async move {
            handle_request(DaemonSocket::new(socket), context).await;
        })
        .await;
    }

    Ok(())
}
