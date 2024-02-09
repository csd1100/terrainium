use anyhow::{anyhow, Context, Result};

use crate::{
    shell::zsh::generate_and_compile,
    types::{
        args::{BiomeArg, Pair},
        biomes::Biome,
        terrain::parse_terrain,
    },
};

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
        std::fs::copy(&toml_file, bkp).context("unable to backup terrain.toml")?;
    }

    let mut terrain = parse_terrain(&toml_file)?;

    if let Some(default) = set_biome {
        terrain
            .update_default_biome(default)
            .context("unable to update default biome")?;
    } else {
        if let Some(biome) = &new {
            terrain
                .add_biome(biome, Biome::new())
                .context("unable to create a new biome")?;
            terrain
                .update(Some(BiomeArg::Value(biome.to_string())), env, alias)
                .context("failed to update newly created biome")?;
        } else {
            terrain
                .update(biome, env, alias)
                .context("failed to update biome")?;
        };
    }

    std::fs::write(toml_file, terrain.to_toml()?)
        .context("failed to write updated terrain.toml")?;

    let central_store = get_central_store_path()?;
    let result: Result<Vec<_>> = terrain
        .into_iter()
        .map(|(biome_name, environment)| {
            generate_and_compile(&central_store, biome_name, environment)
        })
        .collect();

    if result.is_err() {
        return Err(anyhow!(format!(
            "Error while generating and compiling scripts, error: {}",
            result.unwrap_err()
        )));
    }

    return Ok(());
}
