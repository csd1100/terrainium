use anyhow::{Context, Result};
use mockall_double::double;

use crate::{shell::background::start_background_processes, types::args::BiomeArg};

#[double]
use crate::helpers::helpers::fs;

pub fn handle(biome: Option<BiomeArg>) -> Result<()> {
    let terrain = fs::get_parsed_terrain()?
        .get(biome)
        .context("unable to select biome to call destructors")?;
    start_background_processes(terrain.destructors, &terrain.env.unwrap_or_default())
        .context("unable to start background processes")
}
