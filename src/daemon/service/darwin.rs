use crate::common::constants::TERRAINIUMD_TMP_DIR;
use crate::common::constants::{PATH, TERRAINIUMD_DARWIN_SERVICE_FILE};
use crate::daemon::service::Service;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs::write;

pub struct DarwinService {
    path: PathBuf,
}

impl Service for DarwinService {
    fn init(home_dir: &Path) -> Self {
        Self {
            path: home_dir.join(TERRAINIUMD_DARWIN_SERVICE_FILE),
        }
    }

    fn get(&self, daemon_path: Option<PathBuf>, enabled: bool) -> Result<String> {
        let daemon_path =
            daemon_path.unwrap_or(std::env::current_exe().context("failed to get current bin")?);

        if !daemon_path.exists() {
            bail!("{} does not exist", daemon_path.display());
        }

        let service = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
    <dict>
        <key>Label</key>
        <string>com.csd1100.terrainium</string>
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

    async fn install(&self, daemon_path: Option<PathBuf>) -> Result<()> {
        let service = self.get(daemon_path, true)?;
        write(&self.path, &service)
            .await
            .context("failed to write service")?;
        Ok(())
    }

    fn start(&self) {
        todo!()
    }

    fn enable(&self, _daemon_path: Option<PathBuf>) -> Result<()> {
        todo!()
    }

    fn stop(&self) {
        todo!()
    }

    fn disable(&self, _daemon_path: Option<PathBuf>) -> Result<()> {
        todo!()
    }

    fn remove(&self) {
        todo!()
    }
}
