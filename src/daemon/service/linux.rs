use crate::common::constants::{DISABLE, ENABLE};
use crate::common::constants::{PATH, TERRAINIUMD_LINUX_SERVICE, TERRAINIUMD_LINUX_SERVICE_PATH};
use crate::common::execute::{Execute, Executor};
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

    fn install(&self, daemon_path: Option<PathBuf>, start: bool) -> anyhow::Result<()> {
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

        self.load().context("failed to load service")?;

        if start {
            self.start().context("failed to start service")?;
        }

        Ok(())
    }

    fn is_loaded(&self) -> anyhow::Result<bool> {
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

    fn load(&self) -> anyhow::Result<()> {
        if self.is_loaded()? {
            println!("service is already loaded");
            return Ok(());
        }

        // reload systemd to load service
        self.reload().context("failed to reload service")
    }

    fn unload(&self) -> anyhow::Result<()> {
        self.reload().context("failed to reload the service")
    }

    fn remove(&self) -> anyhow::Result<()> {
        std::fs::remove_file(&self.path).context("failed to remove service file")?;
        self.unload().context("failed to unload")?;
        Ok(())
    }

    fn enable(&self, now: bool) -> anyhow::Result<()> {
        if !self.is_loaded()? {
            bail!(
                "service is not loaded, re-run terrainiumd install-service to install and load the service"
            );
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

    fn disable(&self) -> anyhow::Result<()> {
        if !self.is_loaded()? {
            bail!(
                "service is not loaded, re-run terrainiumd install-service to install and load the service"
            );
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

    fn is_running(&self) -> anyhow::Result<bool> {
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

    fn start(&self) -> anyhow::Result<()> {
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

    fn stop(&self) -> anyhow::Result<()> {
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
            std::fs::create_dir_all(&path).expect("failed to create services directory");
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
