use anyhow::{bail, Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use terrainium::common::constants::{
    TERRAINIUMD_PID_FILE, TERRAINIUMD_SOCKET, TERRAINIUMD_TMP_DIR,
};
use terrainium::common::execute::{Execute, Executor};
use terrainium::common::types::command::Command;
use terrainium::common::types::styles::{error, warning};
use terrainium::daemon::args::{DaemonArgs, Verbs};
use terrainium::daemon::handlers::handle_request;
use terrainium::daemon::logging::init_logging;
use terrainium::daemon::service::{Service, ServiceProvider};
use terrainium::daemon::types::config::DaemonConfig;
use terrainium::daemon::types::context::DaemonContext;
use terrainium::daemon::types::daemon::Daemon;
use terrainium::daemon::types::daemon_socket::DaemonSocket;
use tokio::signal::unix::{signal, SignalKind};
use tokio_stream::wrappers::UnixListenerStream;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use tracing::metadata::LevelFilter;
use tracing::{debug, info, trace, warn};

fn get_daemon_config() -> DaemonConfig {
    let config = DaemonConfig::from_file().unwrap_or_default();
    debug!("config: {config:#?}");
    config
}

fn is_user_root(executor: Arc<Executor>) -> bool {
    let user = executor.get_output(
        None,
        Command::new(
            "whoami".to_string(),
            vec![],
            Some(PathBuf::from(TERRAINIUMD_TMP_DIR)),
        ),
    );

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

async fn get_daemon_context(
    is_user_root: bool,
    daemon_config: DaemonConfig,
    executor: Arc<Executor>,
    cancellation_token: CancellationToken,
) -> DaemonContext {
    let context = DaemonContext::new(
        is_user_root,
        daemon_config,
        executor.clone(),
        cancellation_token,
        TERRAINIUMD_TMP_DIR,
    )
    .await;
    context.setup_state_manager();
    context
}

async fn start_listening(
    context: Arc<DaemonContext>,
    listener: &mut UnixListenerStream,
) -> Result<()> {
    while let Some(socket) = listener.next().await.transpose()? {
        trace!("received socket connection");
        let context = context.clone();
        let _ = tokio::spawn(async move {
            handle_request(context, DaemonSocket::new(socket)).await;
        })
        .await;
    }
    Ok(())
}

async fn run(
    args: DaemonArgs,
    is_root: bool,
    config: DaemonConfig,
    executor: Arc<Executor>,
    cancellation_token: CancellationToken,
) -> Result<()> {
    let (subscriber, (_file_guard, _out_guard)) = init_logging(
        TERRAINIUMD_TMP_DIR,
        LevelFilter::from(args.options.log_level),
    );
    tracing::subscriber::set_global_default(subscriber).expect("unable to set global subscriber");

    if args.options.create_config {
        return DaemonConfig::create_file().context("failed to create terrainiumd config");
    }

    // write pid
    std::fs::write(TERRAINIUMD_PID_FILE, std::process::id().to_string())
        .context("failed to write terrainiumd pid file")?;

    let context = Arc::new(get_daemon_context(is_root, config, executor, cancellation_token).await);
    let token = context.cancellation_token();

    let mut daemon = Daemon::new(PathBuf::from(TERRAINIUMD_SOCKET), args.options.force)
        .await
        .context("to create start the terrainium daemon")?;

    tokio::select! {
        _ = token.cancelled() => {
            info!("stopping terrainium daemon");
            Ok(())
        }

        res = start_listening(context.clone(), daemon.listener()) => {
            res
        }
    }
}

async fn start() -> Result<()> {
    if cfg!(debug_assertions) {
        println!("{}: you are running debug build of terrainiumd, which might cause some unwanted behavior.",warning("WARNING"));
    }

    let args = DaemonArgs::parse();

    let config = get_daemon_config();
    let executor = Arc::new(Executor);
    let is_root = is_user_root(executor.clone());

    if is_root && !config.is_root_allowed() {
        bail!("exiting as service was started as root without being configured.");
    }

    match args.verbs {
        Some(verbs) => {
            let service =
                ServiceProvider::get(executor.clone()).context("failed to get service provider")?;
            match verbs {
                Verbs::InstallService { daemon_path, start } => {
                    service
                        .install(daemon_path, start)
                        .context("failed to install service")?;
                }
                Verbs::RemoveService => {
                    service.remove().context("failed to remove service")?;
                }
                Verbs::EnableService { now } => {
                    service.enable(now).context("failed to enable service")?;
                }
                Verbs::DisableService => {
                    service.disable().context("failed to disable service")?;
                }
                Verbs::StartService => {
                    service.start().context("failed to start service")?;
                }
                Verbs::StopService => {
                    service.stop().context("failed to stop service")?;
                }
                Verbs::Status => {
                    service.status().context("failed to get service status")?;
                }
            }
            Ok(())
        }
        None => {
            let token = CancellationToken::new();
            let cloned_token = token.clone();

            let mut sigterm = signal(SignalKind::terminate())?;
            let result = tokio::join!(
                tokio::spawn(async move {
                    sigterm.recv().await;
                    token.cancel();
                }),
                run(args, is_root, config, executor, cloned_token)
            );
            result.1
        }
    }
}

#[tokio::main]
async fn main() {
    match start().await {
        Ok(_) => {}
        Err(err) => {
            let err = format!("exiting terrainiumd with an error: {err:?}");
            eprintln!("{}: {err}", error("ERROR"));
        }
    }
}
