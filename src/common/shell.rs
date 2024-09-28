use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
#[double]
use crate::common::execute::Run;
use anyhow::Result;
use mockall_double::double;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::process::Output;

pub mod zsh;

pub trait Shell: Debug + PartialEq {
    fn get() -> Self;
    fn exe(&self) -> String;
    fn runner(&self) -> Run;
    fn update_rc(data: String) -> Result<()>;
    fn generate_scripts(&self, context: &Context, terrain: Terrain) -> Result<()>;
    fn execute(&self, args: Vec<String>, envs: Option<BTreeMap<String, String>>) -> Result<Output>;
    fn templates() -> BTreeMap<String, String>;
}

#[derive(Debug, PartialEq)]
pub struct Zsh {
    exe: String,
    runner: Run,
}
