use crate::common::constants::TERRAINIUMD_TMP_DIR;
use crate::common::constants::{PATH, TERRAINIUMD_DARWIN_SERVICE_FILE};
use crate::common::execute::{Execute, Executor};
use crate::common::types::command::Command;
use crate::daemon::service::Service;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const GUI: &str = "gui";
const LAUNCHCTL: &str = "launchctl";
const BOOTSTRAP: &str = "bootstrap";
const BOOTOUT: &str = "bootout";
const PRINT: &str = "print";
const PROJECT_ID: &str = "com.csd1100.terrainium";

pub struct DarwinService {
    path: PathBuf,
    executor: Arc<Executor>,
}

impl Service for DarwinService {
    fn init(home_dir: &Path, executor: Arc<Executor>) -> Self {
        Self {
            path: home_dir.join(TERRAINIUMD_DARWIN_SERVICE_FILE),
            executor,
        }
    }

    fn is_installed(&self) -> Result<bool> {
        let command = Command::new(
            LAUNCHCTL.to_string(),
            vec![
                PRINT.to_string(),
                self.get_service_target()
                    .context("failed to get service target")?,
            ],
            None,
        );

        let status = self
            .executor
            .wait(None, command, true)
            .context("failed to check if service is installed")?;

        Ok(self.path.exists() && status.success())
    }

    fn install(&self, daemon_path: Option<PathBuf>) -> Result<()> {
        if self.is_installed()? {
            println!("service is already installed!");
            return Ok(());
        }

        let daemon_path =
            daemon_path.unwrap_or(std::env::current_exe().context("failed to get current bin")?);

        let service = self.get(daemon_path, true)?;
        std::fs::write(&self.path, &service).context("failed to write service")?;

        // bootstrap service
        let command = Command::new(
            LAUNCHCTL.to_string(),
            vec![
                BOOTSTRAP.to_string(),
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

    fn start(&self) {
        todo!()
    }

    fn enable(&self) -> Result<()> {
        todo!()
    }

    fn stop(&self) {
        todo!()
    }

    fn disable(&self) -> Result<()> {
        todo!()
    }

    fn remove(&self) -> Result<()> {
        if !self.is_installed()? {
            println!("service is not installed!");
            return Ok(());
        }

        // bootout service
        let command = Command::new(
            LAUNCHCTL.to_string(),
            vec![
                BOOTOUT.to_string(),
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

        std::fs::remove_file(&self.path).context("failed to remove service file")
    }

    fn get(&self, daemon_path: PathBuf, enabled: bool) -> Result<String> {
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
        <{enabled}/>
        <key>KeepAlive</key>
        <dict>
            <key>SuccessfulExit</key>
            <false/>
            <key>Crashed</key>
            <true/>
        </dict>
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
