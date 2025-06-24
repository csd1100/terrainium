#[mockall_double::double]
use crate::common::execute::Executor;
use crate::daemon::service::darwin::DarwinService;
use crate::daemon::service::linux::LinuxService;
use anyhow::{bail, Context, Result};
use home::home_dir;
use std::sync::Arc;

const ERROR_SERVICE_NOT_INSTALLED: &str =
    "service is not installed, run `terrainiumd install-service` to install the service.";
const ERROR_ALREADY_RUNNING: &str = "service is already running!";
const ERROR_IS_NOT_RUNNING: &str = "service is not running!";

pub mod darwin;
pub mod linux;

pub trait Service {
    fn is_installed(&self) -> bool;
    fn install(&self) -> Result<()>;
    fn is_loaded(&self) -> Result<bool>;
    fn load(&self) -> Result<()>;
    fn unload(&self) -> Result<()>;
    fn remove(&self) -> Result<()>;
    fn is_enabled(&self) -> Result<bool>;
    fn enable(&self, now: bool) -> Result<()>;
    fn disable(&self, now: bool) -> Result<()>;
    fn is_running(&self) -> Result<bool>;
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn status(&self) -> Result<String> {
        if !self.is_installed() {
            return Ok("not installed".to_string());
        }

        if !self.is_loaded()? {
            return Ok("not loaded".to_string());
        }

        let is_enabled = if self
            .is_enabled()
            .context("failed to check if service is enabled")?
        {
            "enabled"
        } else {
            "disabled"
        };

        if !self.is_running()? {
            Ok(format!("not running ({is_enabled})"))
        } else {
            Ok(format!("running ({is_enabled})"))
        }
    }
    fn get(&self, enabled: bool) -> Result<String>;
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
            DarwinService::init(&home_dir, executor)
        } else if std::env::consts::OS == "linux" {
            LinuxService::init(&home_dir, executor)
        } else {
            bail!("unsupported operating system: {}", std::env::consts::OS);
        }
    }
}

#[cfg(test)]
mod tests {
    pub enum Status {
        Running,
        NotRunning,
        NotLoaded,
        NotInstalled,
    }
}
