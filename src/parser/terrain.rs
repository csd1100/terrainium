use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::types::terrain::Terrain;

pub fn parse_terrain(path: PathBuf) -> Result<Terrain> {
    let terrain = std::fs::read_to_string(path).context("Unable to read file")?;
    let terrain: Terrain = toml::from_str(&terrain).context("Unable to parse terrain")?;
    return Ok(terrain);
}
