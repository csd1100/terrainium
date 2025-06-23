use crate::common::execute::Execute;
#[mockall_double::double]
use crate::common::execute::Executor;
use crate::common::types::command::Command;
use crate::daemon::types::context::DaemonContext;
use anyhow::{bail, Context, Result};
use std::fs::{create_dir_all, remove_file};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tracing::{debug, info, warn};

#[derive(Debug)]
pub struct Daemon {
    listener: UnixListenerStream,
}

const STATUS: usize = 0;
const SIGKILL: usize = 9;

fn kill_command(executor: Arc<Executor>, code: usize, pid: &str) -> Result<()> {
    let command = Command::new(
        "kill".to_string(),
        vec![format!("-{code}"), pid.to_string()],
        Some(std::env::temp_dir()),
    );

    debug!("running command {command}");

    let kill = executor
        .get_output(None, command)
        .context("failed to run kill command")?;

    if !kill.status.success() {
        bail!(
            "kill command failed due to an error: {}",
            String::from_utf8_lossy(&kill.stderr)
        );
    }

    Ok(())
}

fn get_pid(pid_file: PathBuf) -> Result<String> {
    if !pid_file.exists() {
        bail!("pid file doesn't exist");
    }
    let pid = std::fs::read_to_string(&pid_file).context("failed to read pid file")?;
    Ok(pid)
}

fn is_already_running(executor: Arc<Executor>, pid: &str) -> bool {
    kill_command(executor, STATUS, pid).is_ok()
}

fn cleanup(
    executor: Arc<Executor>,
    force: bool,
    socket: &PathBuf,
    pid_file: PathBuf,
) -> Result<()> {
    if let Ok(pid) = get_pid(pid_file) {
        if is_already_running(executor.clone(), &pid) {
            warn!("terrainiumd is already running, pid: {pid}");
            if !force {
                bail!("terrainiumd is already running, and --force is not passed");
            }
            kill_command(executor, SIGKILL, &pid).context("failed to kill terrainiumd")?;
        }
    }
    remove_file(socket).context("failed to remove socket")
}

impl Daemon {
    pub async fn new(context: Arc<DaemonContext>, force: bool) -> Result<Daemon> {
        let dir = context.state_paths().dir();
        let socket = context.state_paths().socket();

        if !socket.is_absolute() {
            bail!("path for socket should be absolute");
        }

        if !dir.exists() {
            info!("creating directories required for socket: {socket:?}");
            create_dir_all(dir).expect("creating parent directory");
        } else if socket.exists() {
            cleanup(
                context.executor(),
                force,
                &socket,
                context.state_paths().pid(),
            )
            .context("failed to clean up old socket")?;
        }

        info!("creating daemon on path: {socket:?}");
        let listener = UnixListenerStream::new(
            UnixListener::bind(socket).context("failed to bind to socket")?,
        );

        // write pid
        std::fs::write(context.state_paths().pid(), std::process::id().to_string())
            .context("failed to write terrainiumd pid file")?;

        Ok(Daemon { listener })
    }

    pub fn listener(&mut self) -> &mut UnixListenerStream {
        &mut self.listener
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::test_utils::assertions::executor::{AssertExecutor, ExpectedCommand};
    use crate::common::types::paths::DaemonPaths;
    use anyhow::Result;
    use std::fs::{metadata, read_to_string};
    use std::os::unix::fs::FileTypeExt;
    use tempfile::tempdir;

    #[tokio::test]
    async fn socket_is_created() -> Result<()> {
        let state_dir = tempdir()?;

        let context = Arc::new(
            DaemonContext::new(
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                DaemonPaths::new(state_dir.path().to_str().unwrap()),
            )
            .await,
        );

        Daemon::new(context, false).await?;

        assert!(state_dir.path().join("socket").exists());
        assert!(metadata(state_dir.path().join("socket"))?
            .file_type()
            .is_socket());
        assert_ne!(read_to_string(state_dir.path().join("pid"))?, "pid");

        Ok(())
    }

    #[tokio::test]
    async fn socket_is_created_when_socket_exist_but_no_pid() -> Result<()> {
        let state_dir = tempdir()?;
        std::fs::write(state_dir.path().join("socket"), "test")?;
        assert!(!metadata(state_dir.path().join("socket"))?
            .file_type()
            .is_socket());

        let context = Arc::new(
            DaemonContext::new(
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                DaemonPaths::new(state_dir.path().to_str().unwrap()),
            )
            .await,
        );

        Daemon::new(context, false).await?;

        assert!(metadata(state_dir.path().join("socket"))?
            .file_type()
            .is_socket());
        assert_ne!(read_to_string(state_dir.path().join("pid"))?, "pid");

        Ok(())
    }

    #[tokio::test]
    async fn socket_is_created_when_socket_exist_but_process_not_running() -> Result<()> {
        let state_dir = tempdir()?;
        std::fs::write(state_dir.path().join("socket"), "test")?;
        std::fs::write(state_dir.path().join("pid"), "pid")?;

        let executor = AssertExecutor::to()
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(
                        "kill".to_string(),
                        vec!["-0".to_string(), "pid".to_string()],
                        Some(std::env::temp_dir()),
                    ),
                    exit_code: 1,
                    should_fail_to_execute: true,
                    output: "".to_string(),
                },
                1,
            )
            .successfully();

        let context = Arc::new(
            DaemonContext::new(
                Default::default(),
                Default::default(),
                Arc::new(executor),
                Default::default(),
                DaemonPaths::new(state_dir.path().to_str().unwrap()),
            )
            .await,
        );

        Daemon::new(context, false).await?;

        assert!(metadata(state_dir.path().join("socket"))?
            .file_type()
            .is_socket());
        assert_ne!(read_to_string(state_dir.path().join("pid"))?, "pid");

        Ok(())
    }

    #[tokio::test]
    async fn socket_is_created_when_socket_exist_process_running_but_force() -> Result<()> {
        let state_dir = tempdir()?;
        std::fs::write(state_dir.path().join("socket"), "test")?;
        std::fs::write(state_dir.path().join("pid"), "pid")?;

        // check if process is running
        let executor = AssertExecutor::to()
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(
                        "kill".to_string(),
                        vec!["-0".to_string(), "pid".to_string()],
                        Some(std::env::temp_dir()),
                    ),
                    exit_code: 0,
                    should_fail_to_execute: false,
                    output: "".to_string(),
                },
                1,
            )
            .successfully();

        // kill the process
        let executor = AssertExecutor::with(executor)
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(
                        "kill".to_string(),
                        vec!["-9".to_string(), "pid".to_string()],
                        Some(std::env::temp_dir()),
                    ),
                    exit_code: 0,
                    should_fail_to_execute: false,
                    output: "".to_string(),
                },
                1,
            )
            .successfully();

        let context = Arc::new(
            DaemonContext::new(
                Default::default(),
                Default::default(),
                Arc::new(executor),
                Default::default(),
                DaemonPaths::new(state_dir.path().to_str().unwrap()),
            )
            .await,
        );

        Daemon::new(context, true).await?;

        assert!(metadata(state_dir.path().join("socket"))?
            .file_type()
            .is_socket());
        assert_ne!(read_to_string(state_dir.path().join("pid"))?, "pid");

        Ok(())
    }

    #[tokio::test]
    async fn throws_an_error_no_force() -> Result<()> {
        let state_dir = tempdir()?;
        std::fs::write(state_dir.path().join("socket"), "test")?;
        std::fs::write(state_dir.path().join("pid"), "pid")?;

        // check if process is running
        let executor = AssertExecutor::to()
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(
                        "kill".to_string(),
                        vec!["-0".to_string(), "pid".to_string()],
                        Some(std::env::temp_dir()),
                    ),
                    exit_code: 0,
                    should_fail_to_execute: false,
                    output: "".to_string(),
                },
                1,
            )
            .successfully();

        let context = Arc::new(
            DaemonContext::new(
                Default::default(),
                Default::default(),
                Arc::new(executor),
                Default::default(),
                DaemonPaths::new(state_dir.path().to_str().unwrap()),
            )
            .await,
        );

        let error = Daemon::new(context, false)
            .await
            .expect_err("expected error")
            .to_string();

        assert_eq!(error, "failed to clean up old socket");

        Ok(())
    }
}
