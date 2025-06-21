#[mockall_double::double]
use crate::common::execute::Executor;
use crate::daemon::service::darwin::DarwinService;
use crate::daemon::service::linux::LinuxService;
use anyhow::{bail, Result};
use home::home_dir;
use std::path::PathBuf;
use std::sync::Arc;

pub mod darwin;
pub mod linux;

pub trait Service {
    fn is_installed(&self) -> bool;
    fn install(&self, daemon_path: Option<PathBuf>) -> Result<()>;
    fn is_loaded(&self) -> Result<bool>;
    fn load(&self) -> Result<()>;
    fn unload(&self) -> Result<()>;
    fn remove(&self) -> Result<()>;
    fn enable(&self, now: bool) -> Result<()>;
    fn disable(&self) -> Result<()>;
    fn is_running(&self) -> Result<bool>;
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn status(&self) -> Result<&'static str> {
        if self.is_installed() {
            if self.is_loaded()? {
                if self.is_running()? {
                    Ok("running")
                } else {
                    Ok("not running")
                }
            } else {
                Ok("not loaded")
            }
        } else {
            Ok("not installed")
        }
    }
    fn get(&self, daemon_path: PathBuf) -> Result<String>;
}

pub struct ServiceProvider;

impl ServiceProvider {
    pub fn get(executor: Arc<Executor>) -> Result<Box<dyn Service>> {
        let home_dir = home_dir();
        if home_dir.is_none() {
            bail!("could not find home directory");
        }
        let home_dir = home_dir.unwrap();

        if std::env::consts::OS == "macos" {
            Ok(DarwinService::init(&home_dir, executor))
        } else if std::env::consts::OS == "linux" {
            Ok(LinuxService::init(&home_dir, executor))
        } else {
            bail!("unsupported operating system: {}", std::env::consts::OS);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::common::constants::TERRAINIUMD;
    use crate::common::test_utils::TEST_DIRECTORY;
    use anyhow::Context;
    use std::path::{Path, PathBuf};

    pub(crate) fn create_test_daemon_binary() -> anyhow::Result<PathBuf> {
        std::fs::create_dir_all(TEST_DIRECTORY)?;
        let test_daemon = Path::new(TEST_DIRECTORY).join(TERRAINIUMD);
        std::fs::write(&test_daemon, "")?;
        Ok(test_daemon)
    }

    pub(crate) fn cleanup_test_daemon_binary() -> anyhow::Result<()> {
        std::fs::remove_dir_all(TEST_DIRECTORY).context("failed to clean up test directory")
    }
}
