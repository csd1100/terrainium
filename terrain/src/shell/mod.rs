use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use terrainium_lib::command::Command;
use terrainium_lib::executor::{Execute, Executor};

use crate::constants::{HOME, SHELL, ZSH};

pub mod zsh;

pub trait Shell: Debug {
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
pub fn get_shell(dir: &Path, executor: Arc<dyn Execute>) -> Result<Box<dyn Shell>> {
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

#[derive(Debug)]
pub struct Zsh {
    bin: String,
    cwd: PathBuf,
    executor: Arc<dyn Execute>,
}

#[cfg(test)]
#[serial_test::serial]
mod tests {
    use std::env::VarError;
    use std::path::Path;
    use std::sync::Arc;

    use pretty_assertions::assert_eq;
    use terrainium_lib::test_utils::{restore_env_var, set_env_var};

    use super::*;

    #[test]
    fn get_shell_errors_if_no_shell_env() {
        let shell: std::result::Result<String, VarError>;
        unsafe {
            shell = set_env_var(SHELL, None);
        }

        let err = get_shell(Path::new(""), Arc::new(Executor))
            .unwrap_err()
            .to_string();

        assert_eq!(err, "failed to detect shell!");

        unsafe {
            restore_env_var(SHELL, shell);
        }
    }

    #[test]
    fn get_shell_errors_if_unsupported_shell() {
        let shell: std::result::Result<String, VarError>;
        unsafe {
            shell = set_env_var(SHELL, Some("/bin/bash"));
        }

        let err = get_shell(Path::new(""), Arc::new(Executor))
            .unwrap_err()
            .to_string();

        assert_eq!(err, "shell \"/bin/bash\" is not supported!");

        unsafe {
            restore_env_var(SHELL, shell);
        }
    }
}
