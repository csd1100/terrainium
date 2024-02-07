use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

use crate::{
    handlers::helpers::{
        create_config_dir, get_central_store_path, get_central_terrain_path, get_local_terrain_path,
    },
    shell::{editor::edit_file, zsh::generate_and_compile},
    types::terrain::Terrain,
};

pub fn handle_init(central: bool, full: bool, edit: bool) -> Result<()> {
    let terrain_toml_path: PathBuf;

    if central {
        create_config_dir()?;
        terrain_toml_path =
            get_central_terrain_path().context("unable to get central toml path")?;
    } else {
        terrain_toml_path = get_local_terrain_path().context("unable to get terrain.toml path")?;
    }

    if !Path::try_exists(&terrain_toml_path.as_path())? {
        let terrain: Terrain;
        if full {
            terrain = Terrain::default();
        } else {
            terrain = Terrain::new();
        }
        std::fs::write(&terrain_toml_path, terrain.to_toml()?)?;

        println!(
            "terrain created at path {}",
            terrain_toml_path.to_string_lossy().to_string()
        );

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

        if edit {
            println!("editing...");
            edit_file(terrain_toml_path)?;
        }
    } else {
        return Err(anyhow!(
            "terrain for this project is already present. edit existing with `terrain edit` command."
        ));
    }
    return Ok(());
}
