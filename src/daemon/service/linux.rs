use crate::common::constants::{DISABLE, ENABLE};
use crate::common::constants::{PATH, TERRAINIUMD_LINUX_SERVICE, TERRAINIUMD_LINUX_SERVICE_PATH};
use crate::common::execute::Execute;
#[mockall_double::double]
use crate::common::execute::Executor;
use crate::common::types::command::Command;
use crate::daemon::service::Service;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const SYSTEMCTL: &str = "systemctl";
const USER: &str = "--user";
const STATUS: &str = "status";
const RELOAD: &str = "daemon-reload";
const NOW: &str = "--now";
const IS_ACTIVE: &str = "is-active";
const START: &str = "start";
const STOP: &str = "stop";

pub struct LinuxService {
    path: PathBuf,
    executor: Arc<Executor>,
}

impl Service for LinuxService {
    fn is_installed(&self) -> bool {
        self.path.exists()
    }

    fn install(&self, daemon_path: Option<PathBuf>) -> Result<()> {
        if self.is_installed() {
            println!("service is already installed!");
            if !self.is_loaded()? {
                println!("loading the service!");
                self.load().context("failed to load the service")?;
            }
            return Ok(());
        }

        let daemon_path =
            daemon_path.unwrap_or(std::env::current_exe().context("failed to get current bin")?);

        let service = self.get(daemon_path)?;
        std::fs::write(&self.path, &service).context("failed to write service")?;

        self.start().context("failed to start service")?;

        Ok(())
    }

    fn is_loaded(&self) -> Result<bool> {
        if !self.is_installed() {
            bail!(
                "service is not installed, run terrainiumd install-service to install the service"
            );
        }

        let command = Command::new(
            SYSTEMCTL.to_string(),
            vec![
                USER.to_string(),
                STATUS.to_string(),
                TERRAINIUMD_LINUX_SERVICE.to_string(),
            ],
            None,
        );

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute process")?;

        let error = String::from_utf8_lossy(&output.stderr);
        Ok(error.is_empty())
    }

    fn load(&self) -> Result<()> {
        if self.is_loaded()? {
            println!("service is already loaded");
            return Ok(());
        }

        // reload systemd to load service
        self.reload().context("failed to reload service")
    }

    fn unload(&self) -> Result<()> {
        self.reload().context("failed to reload the service")
    }

    fn remove(&self) -> Result<()> {
        std::fs::remove_file(&self.path).context("failed to remove service file")?;
        self.unload().context("failed to unload")?;
        Ok(())
    }

    fn enable(&self, now: bool) -> Result<()> {
        if !self.is_loaded()? {
            self.load().context("failed to load the service")?;
        }

        let mut args = vec![
            USER.to_string(),
            ENABLE.to_string(),
            TERRAINIUMD_LINUX_SERVICE.to_string(),
        ];

        if now {
            args.push(NOW.to_string());
        }

        // enable service
        let command = Command::new(SYSTEMCTL.to_string(), args, None);

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute process")?;

        if !output.status.success() {
            bail!(
                "failed to enable service: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    fn disable(&self) -> Result<()> {
        if !self.is_loaded()? {
            self.load().context("failed to load the service")?;
        }

        // disable service
        let command = Command::new(
            SYSTEMCTL.to_string(),
            vec![
                USER.to_string(),
                DISABLE.to_string(),
                TERRAINIUMD_LINUX_SERVICE.to_string(),
            ],
            None,
        );

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute process")?;

        if !output.status.success() {
            bail!(
                "failed to disable service: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    fn is_running(&self) -> Result<bool> {
        if !self.is_loaded()? {
            bail!(
                "service is not loaded, re-run terrainiumd install-service to install and load the service"
            );
        }

        // let pid_file = Path::new(TERRAINIUMD_PID_FILE);
        // if !pid_file.exists() {
        //     return Ok(false);
        // }
        //
        // let pid =
        //     std::fs::read_to_string(pid_file).context("failed to read terrainiumd pid file")?;

        let is_running = Command::new(
            SYSTEMCTL.to_string(),
            vec![
                USER.to_string(),
                IS_ACTIVE.to_string(),
                TERRAINIUMD_LINUX_SERVICE.to_string(),
            ],
            None,
        );

        let running = self
            .executor
            .wait(None, is_running, true)
            .context("failed to check if service is running")?;

        Ok(running.success())
    }

    fn start(&self) -> Result<()> {
        if self.is_running()? {
            bail!("service is already running");
        }

        // start service
        let command = Command::new(
            SYSTEMCTL.to_string(),
            vec![
                USER.to_string(),
                START.to_string(),
                TERRAINIUMD_LINUX_SERVICE.to_string(),
            ],
            None,
        );

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute process")?;

        if !output.status.success() {
            bail!(
                "failed to start the service: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    fn stop(&self) -> Result<()> {
        if !self.is_running()? {
            bail!("service is not running");
        }

        // stop service
        let command = Command::new(
            SYSTEMCTL.to_string(),
            vec![
                USER.to_string(),
                STOP.to_string(),
                TERRAINIUMD_LINUX_SERVICE.to_string(),
            ],
            None,
        );

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute process")?;

        if !output.status.success() {
            bail!(
                "failed to stop the service: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    fn get(&self, daemon_path: PathBuf) -> Result<String> {
        if !daemon_path.exists() {
            bail!("{} does not exist", daemon_path.display());
        }

        let service = format!(
            r#"[Unit]
Description=terrainium daemon
After=multi-user.target

[Service]
ExecStart={} --force
Environment="PATH={}"
KillSignal=SIGTERM
StandardOutput=append:/tmp/terrainiumd.stdout.log
StandardError=append:/tmp/terrainiumd.stderr.log

[Install]
WantedBy=default.target"#,
            daemon_path.display(),
            std::env::var(PATH).context("failed to get PATH")?,
        );
        Ok(service)
    }
}

impl LinuxService {
    pub(crate) fn init(home_dir: &Path, executor: Arc<Executor>) -> Box<dyn Service> {
        let path = home_dir.join(format!(
            "{TERRAINIUMD_LINUX_SERVICE_PATH}/{TERRAINIUMD_LINUX_SERVICE}"
        ));

        if !path.parent().unwrap().exists() {
            std::fs::create_dir_all(path.parent().unwrap())
                .expect("failed to create services directory");
        }

        Box::new(Self { path, executor })
    }

    fn reload(&self) -> Result<()> {
        let command = Command::new(
            SYSTEMCTL.to_string(),
            vec![USER.to_string(), RELOAD.to_string()],
            None,
        );

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute process")?;

        if !output.status.success() {
            bail!(
                "failed to load the service: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // use crate::client::test_utils::{restore_env_var, set_env_var};
    // use crate::common::constants::{
    //     PATH, TERRAINIUMD_DARWIN_SERVICE_FILE, TERRAINIUMD_LINUX_SERVICE,
    //     TERRAINIUMD_LINUX_SERVICE_PATH,
    // };
    // use crate::common::execute::MockExecutor;
    // use crate::daemon::service::linux::LinuxService;
    // use crate::daemon::service::tests::{cleanup_test_daemon_binary, create_test_daemon_binary};
    // use anyhow::Result;
    // use serial_test::serial;
    // use std::env::VarError;
    // use std::path::PathBuf;
    // use std::sync::Arc;
    // use tempfile::tempdir;
    //
    // #[test]
    // fn install_works() -> Result<()> {
    //     let home_dir = tempdir()?;
    //     let service = LinuxService::init(home_dir.path(), Arc::new(MockExecutor::new()));
    //     service.install(None)?;
    //     assert!(home_dir
    //         .path()
    //         .join(format!(
    //             "{TERRAINIUMD_LINUX_SERVICE_PATH}/{TERRAINIUMD_LINUX_SERVICE}"
    //         ))
    //         .exists());
    //     assert!(service.is_installed());
    //     Ok(())
    // }
    //
    // #[serial]
    // #[test]
    // fn install_with_daemon_path() -> Result<()> {
    //     let path: Result<String, VarError>;
    //     unsafe { path = set_env_var(PATH, Some("/usr/local/bin:/usr/bin:/bin")) }
    //     let home_dir = tempdir()?;
    //
    //     // create daemon file
    //
    //     let service = LinuxService::init(home_dir.path(), Arc::new(MockExecutor::new()));
    //     service.install(Some(create_test_daemon_binary()?))?;
    //     assert!(home_dir
    //         .path()
    //         .join(format!(
    //             "{TERRAINIUMD_LINUX_SERVICE_PATH}/{TERRAINIUMD_LINUX_SERVICE}"
    //         ))
    //         .exists());
    //     assert!(service.is_installed());
    //
    //     let contents =
    //         std::fs::read_to_string(home_dir.path().join(TERRAINIUMD_DARWIN_SERVICE_FILE))?;
    //     let expected = std::fs::read_to_string("./tests/data/com.csd1100.terrainium.plist")?;
    //
    //     assert_eq!(contents, expected);
    //
    //     cleanup_test_daemon_binary()?;
    //     unsafe { restore_env_var(PATH, path) }
    //     Ok(())
    // }
    //
    // #[test]
    // fn install_with_daemon_path_errors_no_daemon() -> Result<()> {
    //     let home_dir = tempdir()?;
    //
    //     let service = LinuxService::init(home_dir.path(), Arc::new(MockExecutor::new()));
    //     let error = service
    //         .install(Some(PathBuf::from("/non_existent")))
    //         .expect_err("expected error")
    //         .to_string();
    //
    //     assert_eq!(error, "/non_existent does not exist");
    //     Ok(())
    // }
}
