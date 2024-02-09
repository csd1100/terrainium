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
        create_config_dir().context("unable to create config directory")?;
        terrain_toml_path =
            get_central_terrain_path().context("unable to get central toml path")?;
    } else {
        terrain_toml_path = get_local_terrain_path().context("unable to get local terrain.toml")?;
    }

    if !Path::try_exists(&terrain_toml_path.as_path())
        .context("failed to validate if terrain already exists")?
    {
        let terrain: Terrain;
        if full {
            terrain = Terrain::default();
        } else {
            terrain = Terrain::new();
        }
        std::fs::write(&terrain_toml_path, terrain.to_toml()?)
            .context("failed to write generated terrain to toml file")?;

        println!(
            "terrain created at path {}",
            terrain_toml_path.to_string_lossy().to_string()
        );

        let central_store = get_central_store_path().context("unable to get central store path")?;
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
            edit_file(&terrain_toml_path).context("failed to edit terrain.toml")?;
        }
    } else {
        return Err(anyhow!(
            "terrain for this project is already present. edit existing with `terrain edit` command."
        ));
    }
    return Ok(());
}
