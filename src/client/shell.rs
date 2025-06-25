use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
#[mockall_double::double]
use crate::common::execute::Executor;
use crate::common::types::command::Command;
use anyhow::{Context as AnyhowContext, Result};
use handlebars::Handlebars;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};
use std::sync::Arc;

pub mod zsh;

pub trait Shell: Debug {
    fn get(cwd: &Path, executor: Arc<Executor>) -> Self;
    fn command(&self) -> Command;
    fn get_init_rc_contents() -> String;
    fn get_integration_script(&self) -> String;
    fn setup_integration(&self, init_script_dir: PathBuf) -> Result<()>;
    fn update_rc(path: Option<PathBuf>) -> Result<()>;
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
    fn generate_envs(&self, context: &Context, biome_arg: &str)
        -> Result<BTreeMap<String, String>>;
    fn templates() -> BTreeMap<String, String>;
}

#[derive(Debug)]
pub struct Zsh {
    bin: String,
    cwd: PathBuf,
    executor: Arc<Executor>,
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
