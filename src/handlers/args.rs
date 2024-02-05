use std::collections::HashMap;

use anyhow::{Context, Result};
use uuid::Uuid;

use crate::{
    handlers::helpers::get_parsed_terrain,
    shell::{
        background::start_background_processes,
        editor::edit_file,
        zsh::{compile, generate_zsh_script, get_zsh_envs, spawn_zsh},
    },
    templates::get::{print_aliases, print_all, print_constructors, print_destructors, print_env},
    types::{
        args::{BiomeArg, GetOpts},
        terrain::parse_terrain,
    },
};

use super::{
    constants::{TERRAINIUM_ENABLED, TERRAINIUM_SESSION_ID},
    helpers::{get_central_store_path, get_terrain_toml, merge_hashmaps},
};

pub fn handle_edit() -> Result<()> {
    let toml_file = get_terrain_toml().context("unable to get terrain.toml path")?;

    edit_file(toml_file)?;

    let terrain = parse_terrain(&get_terrain_toml()?)?;
    let central_terrain_path = get_central_store_path()?;
    generate_zsh_script(&central_terrain_path, terrain.get(None)?)?;
    compile(&central_terrain_path)?;

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

pub fn handle_enter(biome: Option<BiomeArg>) -> Result<()> {
    let terrain = get_parsed_terrain()?.get(biome.clone())?;
    let mut envs = terrain.env;

    if let None = envs {
        envs = Some(HashMap::<String, String>::new());
    }

    if let Some(envs) = envs.as_mut() {
        envs.insert(TERRAINIUM_ENABLED.to_string(), "1".to_string());
        envs.insert(
            TERRAINIUM_SESSION_ID.to_string(),
            Uuid::new_v4().to_string(),
        );
    }

    if let Some(envs) = envs {
        let zsh_env = get_zsh_envs()?;
        let merged = merge_hashmaps(&envs.clone(), &zsh_env);

        handle_construct(biome, Some(&merged))?;
        spawn_zsh(vec!["-s"], Some(merged))?;
    }

    return Ok(());
}

pub fn handle_exit(biome: Option<BiomeArg>) -> Result<()> {
    return handle_deconstruct(biome);
}

pub fn handle_construct(
    biome: Option<BiomeArg>,
    envs: Option<&HashMap<String, String>>,
) -> Result<()> {
    let terrain = get_parsed_terrain()?.get(biome)?;
    if let Some(envs) = envs {
        return start_background_processes(terrain.constructors, envs);
    }
    return start_background_processes(terrain.constructors, &terrain.env.unwrap_or_default());
}

pub fn handle_deconstruct(biome: Option<BiomeArg>) -> Result<()> {
    let terrain = get_parsed_terrain()?.get(biome)?;
    return start_background_processes(terrain.destructors, &terrain.env.unwrap_or_default());
}
