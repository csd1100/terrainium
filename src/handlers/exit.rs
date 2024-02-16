use anyhow::{Context, Result};

use crate::types::args::BiomeArg;

use super::deconstruct;

pub fn handle(biome: Option<BiomeArg>) -> Result<()> {
    deconstruct::handle(biome).context("unable to call destructors")
}
