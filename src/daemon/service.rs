use crate::common::execute::Executor;
use crate::daemon::service::darwin::DarwinService;
use anyhow::{bail, Result};
use home::home_dir;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(target_os = "macos")]
pub mod darwin;
#[cfg(target_os = "linux")]
pub mod linux;

pub trait Service {
    fn init(home_dir: &Path, executor: Arc<Executor>) -> Self;
    fn is_installed(&self) -> Result<bool>;
    fn install(&self, daemon_path: Option<PathBuf>, start: bool) -> Result<()>;
    fn enable(&self, now: bool) -> Result<()>;
    fn disable(&self) -> Result<()>;
    fn is_loaded(&self) -> Result<bool>;
    fn load(&self) -> Result<()>;
    fn unload(&self) -> Result<()>;
    fn is_running(&self) -> Result<bool>;
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn remove(&self) -> Result<()>;
    fn status(&self) -> Result<()>;
    fn get(&self, daemon_path: PathBuf, enabled: bool) -> Result<String>;
}

pub struct ServiceProvider;

impl ServiceProvider {
    pub fn get(executor: Arc<Executor>) -> Result<impl Service> {
        let home_dir = home_dir();
        if home_dir.is_none() {
            bail!("could not find home directory");
        }
        let home_dir = home_dir.unwrap();

        if std::env::consts::OS == "macos" {
            Ok(DarwinService::init(&home_dir, executor))
        } else if std::env::consts::OS == "linux" {
            todo!();
        } else {
            bail!("unsupported operating system: {}", std::env::consts::OS);
        }
    }
}
