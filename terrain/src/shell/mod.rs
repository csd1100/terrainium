use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use terrainium_lib::command::Command;
use terrainium_lib::executor::{Execute, Executor};

use crate::constants::{HOME, SHELL, ZSH};

pub mod zsh;

pub trait Shell {
    fn command(&self) -> Command;
    fn get_init_rc_contents(&self) -> String;
    fn generate_integration_script(&self) -> String;
    fn create_integration_script(&self, init_script_dir: PathBuf) -> Result<()>;
    fn get_default_rc(&self, home_dir: &Path) -> PathBuf;
    fn update_rc(&self, home_dir: &Path, path: PathBuf) -> Result<()>;
    fn execute(
        &self,
        args: Vec<String>,
        envs: Option<Arc<BTreeMap<String, String>>>,
    ) -> Result<Output>;
}

/// get shell instance
pub fn get_shell(dir: &Path, executor: Arc<Executor>) -> Result<Box<dyn Shell>> {
    let shell = std::env::var(SHELL);
    if shell.is_err() {
        bail!("failed to detect shell!");
    }

    let shell = shell.expect("to be present");

    if !shell.to_lowercase().contains(ZSH) {
        bail!("shell \"{shell}\" is not supported!");
    }

    Ok(Box::new(Zsh::get(shell, dir, executor)))
}

/// update rc for the shell
pub fn update_rc(home_dir: &Path, rc_path: Option<PathBuf>) -> Result<()> {
    #[allow(clippy::default_constructed_unit_structs)]
    let shell = get_shell(home_dir, Arc::new(Executor))?;

    let mut rc_path = rc_path.unwrap_or(shell.get_default_rc(home_dir));

    if rc_path.starts_with(HOME) {
        let rc = rc_path
            .strip_prefix(HOME)
            .context("failed to remove '~/' from path")?;
        rc_path = home_dir.join(rc);
    }

    shell
        .update_rc(home_dir, rc_path)
        .context("failed to update rc")
}

pub struct Zsh {
    bin: String,
    cwd: PathBuf,
    executor: Arc<dyn Execute>,
}
