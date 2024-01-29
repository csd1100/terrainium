use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

use crate::{
    parser::helpers::{get_central_terrain_path, get_local_terrain_path},
    types::{
        args::{BiomeArg, Pair},
        terrain::{terrain_to_toml, Terrain},
    },
};

pub fn handle_init(central: bool, full: bool, edit: bool) -> Result<()> {
    let terrain_toml_path: PathBuf;
    let terrain: Terrain;

    if central {
        terrain_toml_path =
            get_central_terrain_path().context("unable to get central toml path")?;
    } else {
        terrain_toml_path = get_local_terrain_path().context("unable to get terrain.toml path")?;
    }

    if !Path::try_exists(&terrain_toml_path.as_path())? {
        if full {
            terrain = Terrain::default();
        } else {
            terrain = Terrain::new();
        }
        std::fs::write(&terrain_toml_path, terrain_to_toml(terrain)?)?;

        println!(
            "terrain created at path {}",
            terrain_toml_path.to_string_lossy().to_string()
        );

        if edit {
            println!("editing...");
            todo!()
        }
    } else {
        return Err(anyhow!(
        "terrain for this project is already present. edit existing with `terrain edit` command."
    ));
    }
    return Ok(());
}

pub fn handle_edit() -> Result<()> {
    todo!()
}

pub fn handle_update(
    _set_biome: Option<String>,
    _biome: Option<BiomeArg>,
    _env: Option<Vec<Pair>>,
    _alias: Option<Vec<Pair>>,
    _construct: Option<Pair>,
    _destruct: Option<Pair>,
) -> Result<()> {
    todo!()
}

pub fn handle_enter(_biome: Option<BiomeArg>) -> Result<()> {
    todo!()
}

pub fn handle_exit() -> Result<()> {
    todo!()
}

pub fn handle_construct(_biome: Option<BiomeArg>) -> Result<()> {
    todo!()
}

pub fn handle_deconstruct(_biome: Option<BiomeArg>) -> Result<()> {
    todo!()
}
