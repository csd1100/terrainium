use anyhow::Result;

use crate::{handlers::helpers::get_parsed_terrain, types::args::BiomeArg};

pub fn handle_edit() -> Result<()> {
    todo!()
}

pub fn handle_get(
    all: bool,
    biome: Option<BiomeArg>,
    _alias: Option<Vec<String>>,
    _env: Option<Vec<String>>,
    _construcors: bool,
    _destructors: bool,
) -> Result<()> {
    let terrain = get_parsed_terrain()?;
    let environment = terrain.get(biome)?;
    if all {
        println!("{:?}", environment);
    } else {
        todo!();
        // let found_alias;
        // let found_env;
        // if let Some(alias) = alias {
        //     found_alias = Some(environment.find_aliases(alias)?);
        // }
        // if let Some(env) = env {
        //     found_env = Some(environment.find_envs(env)?);
        // }
    }
    return Ok(());
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
