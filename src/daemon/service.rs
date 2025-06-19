use crate::daemon::service::darwin::DarwinService;
use anyhow::{bail, Result};
use home::home_dir;
use std::path::{Path, PathBuf};

pub mod darwin;
pub mod linux;

pub trait Service {
    fn init(home_dir: &Path) -> Self;
    fn get(&self, daemon_path: Option<PathBuf>, enabled: bool) -> Result<String>;
    fn install(
        &self,
        daemon_path: Option<PathBuf>,
    ) -> impl std::future::Future<Output = Result<()>>;
    fn start(&self);
    fn enable(&self, daemon_path: Option<PathBuf>) -> Result<()>;
    fn stop(&self);
    fn disable(&self, daemon_path: Option<PathBuf>) -> Result<()>;
    fn remove(&self);
}

pub struct ServiceProvider;

impl ServiceProvider {
    pub fn get() -> Result<impl Service> {
        let home_dir = home_dir();
        if home_dir.is_none() {
            bail!("could not find home directory");
        }
        let home_dir = home_dir.unwrap();

        if std::env::consts::OS == "macos" {
            Ok(DarwinService::init(&home_dir))
        } else if std::env::consts::OS == "linux" {
            todo!();
        } else {
            bail!("unsupported operating system: {}", std::env::consts::OS);
        }
    }
}
