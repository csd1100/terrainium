use anyhow::{Context, Result};
use mockall_double::double;

use crate::types::args::BiomeArg;

#[double]
use super::deconstruct::run;

pub fn handle(biome: Option<BiomeArg>) -> Result<()> {
    run::destructors(biome).context("unable to deconstruct biome")
}
