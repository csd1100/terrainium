use crate::common::constants::{DISABLE, ENABLE, PATH, TERRAINIUMD_DARWIN_SERVICE_FILE};
use crate::common::constants::{TERRAINIUMD_PID_FILE, TERRAINIUMD_TMP_DIR};
use crate::common::execute::{Execute, Executor};
use crate::common::types::command::Command;
use crate::daemon::service::Service;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const GUI: &str = "gui";
const LAUNCHCTL: &str = "launchctl";
const LOAD: &str = "bootstrap";
const UNLOAD: &str = "bootout";
const PRINT: &str = "print";
const START: &str = "kickstart";
const STOP: &str = "kill";
const PROJECT_ID: &str = "com.csd1100.terrainium";
const SIGTERM: &str = "SIGTERM";

pub struct DarwinService {
    path: PathBuf,
    executor: Arc<Executor>,
}

impl Service for DarwinService {
    fn is_installed(&self) -> bool {
        self.path.exists()
    }

    fn install(&self, daemon_path: Option<PathBuf>, start: bool) -> Result<()> {
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

        if start {
            self.load().context("failed to load service")?;
        }

        Ok(())
    }

    fn is_loaded(&self) -> Result<bool> {
        if !self.is_installed() {
            bail!(
                "service is not installed, run terrainiumd install-service to install the service"
            );
        }

        let is_bootstrapped = Command::new(
            LAUNCHCTL.to_string(),
            vec![
                PRINT.to_string(),
                self.get_service_target()
                    .context("failed to get service target")?,
            ],
            None,
        );

        let bootstrapped = self
            .executor
            .wait(None, is_bootstrapped, true)
            .context("failed to check if service is installed")?;

        Ok(bootstrapped.success())
    }

    fn load(&self) -> Result<()> {
        if self.is_loaded()? {
            println!("service is already loaded");
            return Ok(());
        }

        // bootstrap service
        let command = Command::new(
            LAUNCHCTL.to_string(),
            vec![
                LOAD.to_string(),
                self.get_target()?,
                self.path.to_str().unwrap().to_string(),
            ],
            None,
        );

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute process")?;

        if !output.status.success() {
            bail!(
                "failed to bootstrap service: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    fn unload(&self) -> Result<()> {
        if !self.is_loaded()? {
            println!("service is already unloaded");
            return Ok(());
        }

        // bootout service
        let command = Command::new(
            LAUNCHCTL.to_string(),
            vec![
                UNLOAD.to_string(),
                self.get_service_target()
                    .context("failed to get service target")?,
            ],
            None,
        );

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute process")?;

        if !output.status.success() {
            bail!(
                "failed to bootout service: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    fn remove(&self) -> Result<()> {
        if !self.is_loaded()? {
            self.unload().context("failed to unload")?;
        }
        std::fs::remove_file(&self.path).context("failed to remove service file")
    }

    fn enable(&self, now: bool) -> Result<()> {
        if !self.is_loaded()? {
            bail!(
                "service is not loaded, re-run terrainiumd install-service to install and load the service"
            );
        }

        // enable service
        let command = Command::new(
            LAUNCHCTL.to_string(),
            vec![ENABLE.to_string(), self.get_service_target()?],
            None,
        );

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

        if now {
            self.start().context("failed to start service")?;
        }

        Ok(())
    }

    fn disable(&self) -> Result<()> {
        if !self.is_loaded()? {
            bail!(
                "service is not loaded, re-run terrainiumd install-service to install and load the service"
            );
        }

        // enable service
        let command = Command::new(
            LAUNCHCTL.to_string(),
            vec![DISABLE.to_string(), self.get_service_target()?],
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

        let pid_file = Path::new(TERRAINIUMD_PID_FILE);
        if !pid_file.exists() {
            return Ok(false);
        }

        let pid =
            std::fs::read_to_string(pid_file).context("failed to read terrainiumd pid file")?;

        let is_running = Command::new("kill".to_string(), vec!["-0".to_string(), pid], None);

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
            LAUNCHCTL.to_string(),
            vec![START.to_string(), self.get_service_target()?],
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
            LAUNCHCTL.to_string(),
            vec![
                STOP.to_string(),
                SIGTERM.to_string(),
                self.get_service_target()?,
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
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
    <dict>
        <key>Label</key>
        <string>{PROJECT_ID}</string>
        <key>ProgramArguments</key>
        <array>
            <string>{}</string>
            <string>--force</string>
        </array>
        <key>EnvironmentVariables</key>
        <dict>
            <key>PATH</key>
            <string>{}</string>
        </dict>
        <key>RunAtLoad</key>
        <true/>
        <key>StandardOutPath</key>
        <string>{TERRAINIUMD_TMP_DIR}/stdout.log</string>
        <key>StandardErrorPath</key>
        <string>{TERRAINIUMD_TMP_DIR}/stderr.log</string>
        <key>ProcessType</key>
        <string>Background</string>
    </dict>
</plist>"#,
            daemon_path.display(),
            std::env::var(PATH).context("failed to get PATH")?,
        );
        Ok(service)
    }
}

impl DarwinService {
    pub(crate) fn init(home_dir: &Path, executor: Arc<Executor>) -> Box<dyn Service> {
        let path = home_dir.join(TERRAINIUMD_DARWIN_SERVICE_FILE);

        if !path.parent().unwrap().exists() {
            std::fs::create_dir_all(&path).expect("failed to create services directory");
        }

        Box::new(Self { path, executor })
    }

    fn get_uid(&self) -> Result<String> {
        let command = Command::new("id".to_string(), vec!["-u".to_string()], None);
        let output = self.executor.get_output(None, command)?;
        if !output.status.success() {
            bail!(
                "command to get uid exited with error: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let uid = String::from_utf8(output.stdout).context("failed to parse output")?;
        Ok(uid.replace('\n', ""))
    }

    fn get_target(&self) -> Result<String> {
        Ok(format!("{GUI}/{}", self.get_uid()?))
    }

    fn get_service_target(&self) -> Result<String> {
        Ok(format!("{}/{PROJECT_ID}", self.get_target()?))
    }
}
