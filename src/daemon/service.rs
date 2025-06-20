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
    fn install(&self, daemon_path: Option<PathBuf>, start: bool) -> Result<()>;
    fn is_loaded(&self) -> Result<bool>;
    fn load(&self) -> Result<()>;
    fn unload(&self) -> Result<()>;
    fn remove(&self) -> Result<()>;
    fn enable(&self, now: bool) -> Result<()>;
    fn disable(&self) -> Result<()>;
    fn is_running(&self) -> Result<bool>;
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn status(&self) -> Result<()> {
        let status = if self.is_installed() {
            if self.is_loaded()? {
                if self.is_running()? {
                    "running"
                } else {
                    "not running"
                }
            } else {
                "not loaded"
            }
        } else {
            "not installed"
        };
        println!("{status}");
        Ok(())
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
