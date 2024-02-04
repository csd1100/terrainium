use std::collections::HashMap;

use anyhow::{Context, Result};

use crate::{
    handlers::helpers::get_parsed_terrain, shell::editor::edit_file, templates::get::{print_aliases, print_all, print_constructors, print_destructors, print_env}, types::args::{BiomeArg, GetOpts}
};

use super::helpers::get_terrain_toml;

pub fn handle_edit() -> Result<()> {
    let toml_file = get_terrain_toml().context("unable to get terrain.toml path")?;

    edit_file(toml_file)?;

    return Ok(());
}

pub fn handle_get(all: bool, biome: Option<BiomeArg>, opts: GetOpts) -> Result<()> {
    let terrain = get_parsed_terrain()?;
    if opts.is_empty() || all {
        let mut terrain = terrain.get_printable_terrain(biome)?;
        terrain.all = true;
        print_all(terrain)?;
    } else {
        let GetOpts {
            alias_all,
            alias,
            env_all,
            env,
            constructors,
            destructors,
        } = opts;
        let terrain = terrain.get(biome)?;
        if alias_all {
            print_aliases(terrain.alias.to_owned())?;
        } else {
            if let Some(alias) = alias {
                let found_alias = Some(terrain.find_aliases(alias)?);
                let aliases: HashMap<String, String> = found_alias
                    .expect("to be present")
                    .iter()
                    .map(|(k, v)| {
                        if let None = v {
                            return (k.to_string(), "NOT FOUND".to_string());
                        } else {
                            return (
                                k.to_string(),
                                v.to_owned().expect("to be present").to_string(),
                            );
                        }
                    })
                    .collect();
                print_aliases(Some(aliases))?;
            }
        }

        if env_all {
            print_env(terrain.env.to_owned())?;
        } else {
            if let Some(env) = env {
                let found_env = Some(terrain.find_envs(env)?);
                let env: HashMap<String, String> = found_env
                    .expect("to be present")
                    .iter()
                    .map(|(k, v)| {
                        if let None = v {
                            return (k.to_string(), "NOT FOUND".to_string());
                        } else {
                            return (
                                k.to_string(),
                                v.to_owned().expect("to be present").to_string(),
                            );
                        }
                    })
                    .collect();
                print_env(Some(env))?;
            }
        }

        if constructors {
            print_constructors(terrain.constructors)?;
        }
        if destructors {
            print_destructors(terrain.destructors)?;
        }
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
