use anyhow::{Context, Result};

use crate::{shell::zsh::{compile, generate_zsh_script}, types::{
    args::{BiomeArg, Pair},
    biomes::Biome,
    terrain::parse_terrain,
}};

use super::helpers::{get_central_store_path, get_terrain_toml};

pub fn handle_update(
    set_biome: Option<String>,
    new: Option<String>,
    biome: Option<BiomeArg>,
    env: Option<Vec<Pair>>,
    alias: Option<Vec<Pair>>,
    backup: bool,
) -> Result<()> {
    let toml_file = get_terrain_toml().context("unable to get terrain.toml path")?;

    if backup {
        let bkp = toml_file.with_extension("toml.bkp");
        std::fs::copy(&toml_file, bkp)?;
    }

    let mut terrain = parse_terrain(&toml_file)?;

    if let Some(default) = set_biome {
        terrain.update_default_biome(default)?;
    } else {
        if let Some(biome) = &new {
            terrain.add_biome(biome, Biome::new())?;
            terrain.update(Some(BiomeArg::Value(biome.to_string())), env, alias)?;
        } else {
            terrain.update(biome, env, alias)?;
        };
    }

    std::fs::write(toml_file, terrain.to_toml()?)?;

    let central_terrain_path = get_central_store_path()?;
    generate_zsh_script(&central_terrain_path, terrain.get(None)?)?;
    compile(&central_terrain_path)?;

    return Ok(());
}
