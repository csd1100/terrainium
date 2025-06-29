use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};
use std::sync::Arc;

use anyhow::{Context as AnyhowContext, Result, bail};
use handlebars::Handlebars;
use serde::Serialize;

use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use crate::common::constants::{SHELL, ZSH};
#[mockall_double::double]
use crate::common::execute::Executor;
use crate::common::types::command::Command;

pub mod zsh;

pub trait Shell: Debug {
    fn get(cwd: &Path, executor: Arc<Executor>) -> Self;
    fn command(&self) -> Command;
    fn get_init_rc_contents() -> String;
    fn generate_integration_script(&self) -> String;
    fn create_integration_script(&self, init_script_dir: PathBuf) -> Result<()>;
    fn get_default_rc(&self, home_dir: &Path) -> PathBuf;
    fn update_rc(&self, home_dir: &Path, path: PathBuf) -> Result<()>;
    fn generate_scripts(&self, context: &Context, terrain: Terrain) -> Result<()>;
    fn execute(
        &self,
        args: Vec<String>,
        envs: Option<Arc<BTreeMap<String, String>>>,
    ) -> Result<Output>;
    fn spawn(
        &self,
        envs: Option<Arc<BTreeMap<String, String>>>,
    ) -> impl std::future::Future<Output = Result<ExitStatus>> + Send;
    fn generate_envs(
        &self,
        scripts_dir: PathBuf,
        biome_arg: &str,
    ) -> Result<BTreeMap<String, String>>;
    fn templates() -> BTreeMap<String, String>;
}

#[derive(Debug)]
pub struct Zsh {
    bin: String,
    cwd: PathBuf,
    executor: Arc<Executor>,
}

pub fn get_shell(dir: &Path, executor: Arc<Executor>) -> Result<Zsh> {
    let shell = std::env::var(SHELL);
    if shell.is_err() {
        bail!("failed to detect shell!");
    }

    let shell = shell.expect("to be present");

    if !shell.to_lowercase().contains(ZSH) {
        bail!("shell \"{shell}\" is not supported!");
    }
    Ok(Zsh::get(dir, executor))
}

pub fn update_rc(home_dir: &Path, rc_path: Option<PathBuf>) -> Result<()> {
    #[allow(clippy::default_constructed_unit_structs)]
    let shell = get_shell(home_dir, Arc::new(Executor::default()))?;

    let mut rc_path = rc_path.unwrap_or(shell.get_default_rc(home_dir));

    if rc_path.starts_with("~/") {
        let rc = rc_path
            .strip_prefix("~/")
            .context("failed to remove '~/' from path")?;
        rc_path = home_dir.join(rc);
    }

    shell
        .update_rc(home_dir, rc_path)
        .context("failed to update rc")
}

pub(crate) fn render<T: Serialize>(
    main_template: String,
    templates: BTreeMap<String, String>,
    arg: T,
) -> Result<String> {
    let mut handlebars = Handlebars::new();
    templates.iter().for_each(|(name, template)| {
        handlebars
            .register_template_string(name, template)
            .expect("failed to register template")
    });

    handlebars
        .render(&main_template, &arg)
        .context("failed to render template ".to_string() + &main_template)
}

#[cfg(test)]
#[serial_test::serial]
mod tests {
    use std::env::VarError;
    use std::path::Path;
    use std::sync::Arc;

    use pretty_assertions::assert_eq;

    use crate::client::shell::get_shell;
    use crate::client::test_utils::restore_env_var;
    use crate::common::constants::SHELL;
    use crate::common::execute::MockExecutor;

    #[test]
    fn get_shell_errors_if_no_shell_env() {
        let shell: std::result::Result<String, VarError>;
        unsafe {
            shell = crate::client::test_utils::set_env_var(SHELL, None);
        }

        let err = get_shell(Path::new(""), Arc::new(MockExecutor::new()))
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
            shell = crate::client::test_utils::set_env_var(SHELL, Some("/bin/bash"));
        }

        let err = get_shell(Path::new(""), Arc::new(MockExecutor::new()))
            .unwrap_err()
            .to_string();

        assert_eq!(err, "shell \"/bin/bash\" is not supported!");

        unsafe {
            restore_env_var(SHELL, shell);
        }
    }
}
