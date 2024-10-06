use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
#[mockall_double::double]
use crate::common::execute::CommandToRun;
use anyhow::Result;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::PathBuf;
use std::process::{ExitStatus, Output};

pub mod zsh;

pub(crate) trait Shell: Debug + PartialEq {
    fn get() -> Self;
    fn runner(&self) -> CommandToRun;
    fn update_rc(&self, path: Option<PathBuf>) -> Result<()>;
    fn generate_scripts(&self, context: &Context, terrain: Terrain) -> Result<()>;
    fn execute(&self, args: Vec<String>, envs: Option<BTreeMap<String, String>>) -> Result<Output>;
    async fn spawn(&self, envs: BTreeMap<String, String>) -> Result<ExitStatus>;
    fn generate_envs(
        &self,
        context: &Context,
        biome_arg: String,
    ) -> Result<BTreeMap<String, String>>;
    fn templates() -> BTreeMap<String, String>;
}

#[derive(Debug, PartialEq)]
pub struct Zsh {
    exe: String,
    runner: CommandToRun,
}
