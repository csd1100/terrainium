use anyhow::{bail, Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use terrainium::common::constants::{TERRAINIUMD_SOCKET, TERRAINIUMD_TMP_DIR};
use terrainium::common::execute::{Execute, Executor};
use terrainium::common::types::command::Command;
use terrainium::common::types::styles::{error, warning};
use terrainium::daemon::args::DaemonArgs;
use terrainium::daemon::handlers::handle_request;
use terrainium::daemon::logging::init_logging;
use terrainium::daemon::types::config::DaemonConfig;
use terrainium::daemon::types::context::DaemonContext;
use terrainium::daemon::types::daemon::Daemon;
use terrainium::daemon::types::daemon_socket::DaemonSocket;
use tokio_stream::StreamExt;
use tracing::metadata::LevelFilter;
use tracing::{debug, info, trace, warn};

fn get_daemon_config() -> DaemonConfig {
    let config = DaemonConfig::from_file().unwrap_or_default();
    debug!("config: {config:#?}");
    config
}

fn is_user_root(executor: Arc<Executor>) -> bool {
    let user = executor.get_output(Command::new(
        "whoami".to_string(),
        vec![],
        None,
        Some(PathBuf::from(TERRAINIUMD_TMP_DIR)),
    ));

    if let Ok(user) = user {
        let user = String::from_utf8_lossy(&user.stdout);
        info!("running service as user: {}", user.trim());
        if user.contains("root") {
            warn!(
                "running service as root is not advised, see terrainium docs for more information."
            );
            return true;
        }
    } else {
        warn!("could not figure out user running service, running in non-root mode.");
    }
    false
}

async fn get_daemon_context() -> DaemonContext {
    let executor = Arc::new(Executor);
    let context = DaemonContext::new(
        executor.clone(),
        get_daemon_config(),
        TERRAINIUMD_TMP_DIR,
        is_user_root(executor),
    )
    .await;
    context.setup_state_manager();
    context
}

async fn start() -> Result<()> {
    if cfg!(debug_assertions) {
        println!("{}: you are running debug build of terrainiumd, which might cause some unwanted behavior.",warning("WARNING"));
    }

    let args = DaemonArgs::parse();

    let (subscriber, (_file_guard, _out_guard)) =
        init_logging(TERRAINIUMD_TMP_DIR, LevelFilter::from(args.log_level));
    tracing::subscriber::set_global_default(subscriber).expect("unable to set global subscriber");

    if args.create_config {
        return DaemonConfig::create_file().context("failed to create terrainiumd config");
    }

    let context = Arc::new(get_daemon_context().await);

    if context.should_exit_early() {
        bail!("exiting as service was started as root without being configured.");
    }

    let mut daemon = Daemon::new(PathBuf::from(TERRAINIUMD_SOCKET), args.force)
        .await
        .context("to create start the terrainium daemon")?;
    let listener = daemon.listener();

    while let Some(socket) = listener.next().await.transpose()? {
        let context = context.clone();
        trace!("received socket connection");
        let _ = tokio::spawn(async move {
            handle_request(context, DaemonSocket::new(socket)).await;
        })
        .await;
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    match start().await {
        Ok(_) => {
            println!("exiting terrainiumd");
        }
        Err(err) => {
            let err = format!("exiting terrainiumd with an error: {err:?}");
            eprintln!("{}: {err}", error("ERROR"));
        }
    }
}
