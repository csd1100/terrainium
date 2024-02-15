use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use mockall_double::double;
use uuid::Uuid;

use crate::{
    shell::{
        background::start_background_processes,
        zsh::{get_zsh_envs, spawn_zsh},
    },
    templates::get::{print_aliases, print_all, print_constructors, print_destructors, print_env},
    types::{
        args::{BiomeArg, GetOpts},
        terrain::parse_terrain,
    },
};

use super::{
    constants::{TERRAINIUM_ENABLED, TERRAINIUM_SESSION_ID},
    helpers::{get_parsed_terrain, merge_hashmaps},
};

#[double]
use crate::shell::editor::edit;

#[double]
use crate::shell::zsh::ops;

#[double]
use super::helpers::fs;

pub fn handle_edit() -> Result<()> {
    let toml_file = fs::get_terrain_toml().context("unable to get terrain.toml path")?;

    edit::file(&toml_file).context("failed to start editor")?;

    let terrain = parse_terrain(&toml_file)?;
    let central_store = fs::get_central_store_path()?;
    let result: Result<Vec<_>> = terrain
        .into_iter()
        .map(|(biome_name, environment)| {
            ops::generate_and_compile(&central_store, biome_name, environment)
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

pub fn handle_generate() -> Result<()> {
    let terrain = parse_terrain(&fs::get_terrain_toml()?)?;
    let central_store = fs::get_central_store_path()?;
    let result: Result<Vec<_>> = terrain
        .into_iter()
        .map(|(biome_name, environment)| {
            ops::generate_and_compile(&central_store, biome_name, environment)
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

pub fn handle_get(all: bool, biome: Option<BiomeArg>, opts: GetOpts) -> Result<()> {
    let terrain = get_parsed_terrain()?;
    if opts.is_empty() || all {
        let mut terrain = terrain
            .get_printable_terrain(biome)
            .context("failed to get printable terrain")?;
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
            print_aliases(terrain.alias.to_owned()).context("unable to print aliases")?;
        } else {
            if let Some(alias) = alias {
                let found_alias = Some(
                    terrain
                        .find_aliases(alias)
                        .context("unable to get aliases")?,
                );
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
                print_aliases(Some(aliases)).context("unable to print aliases")?;
            }
        }

        if env_all {
            print_env(terrain.env.to_owned()).context("unable to print env vars")?;
        } else {
            if let Some(env) = env {
                let found_env = Some(terrain.find_envs(env).context("unable to get env vars")?);
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
                print_env(Some(env)).context("unable to print env vars")?;
            }
        }

        if constructors {
            print_constructors(terrain.constructors).context("unable to print constructors")?;
        }
        if destructors {
            print_destructors(terrain.destructors).context("unable to print destructors")?;
        }
    }
    return Ok(());
}

pub fn handle_enter(biome: Option<BiomeArg>) -> Result<()> {
    let terrain = get_parsed_terrain()?;
    let selected = terrain
        .get(biome.clone())
        .context("unable to select biome")?;
    let mut envs = selected.env;

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
        let zsh_env = get_zsh_envs(terrain.get_selected_biome_name(&biome)?)
            .context("unable to set zsh environment varibles")?;
        let merged = merge_hashmaps(&envs.clone(), &zsh_env);

        handle_construct(biome, Some(&merged)).context("unable to construct biome")?;
        spawn_zsh(vec!["-s"], Some(merged)).context("unable to start zsh")?;
    }

    return Ok(());
}

pub fn handle_exit(biome: Option<BiomeArg>) -> Result<()> {
    return handle_deconstruct(biome).context("unable to call destructors");
}

pub fn handle_construct(
    biome: Option<BiomeArg>,
    envs: Option<&HashMap<String, String>>,
) -> Result<()> {
    let terrain = get_parsed_terrain()?
        .get(biome)
        .context("unable to select biome to call constructors")?;
    if let Some(envs) = envs {
        return start_background_processes(terrain.constructors, envs)
            .context("unable to start background processes");
    }
    return start_background_processes(terrain.constructors, &terrain.env.unwrap_or_default())
        .context("unable to start background processes");
}

pub fn handle_deconstruct(biome: Option<BiomeArg>) -> Result<()> {
    let terrain = get_parsed_terrain()?
        .get(biome)
        .context("unable to select biome to call destructors")?;
    return start_background_processes(terrain.destructors, &terrain.env.unwrap_or_default())
        .context("unable to start background processes");
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use anyhow::Result;
    use mockall::predicate::eq;
    use serial_test::serial;

    use crate::{
        handlers::helpers::mock_fs,
        shell::{editor::mock_edit, zsh::mock_ops},
        types::{args::BiomeArg, terrain::test_data},
    };

    use super::handle_edit;

    #[test]
    #[serial]
    fn handle_edit_opens_editor_and_compiles_scripts() -> Result<()> {
        let mock_get_toml_path = mock_fs::get_terrain_toml_context();
        mock_get_toml_path
            .expect()
            .return_once(|| Ok(PathBuf::from("./example_configs/terrain.full.toml")))
            .times(1);

        let mock_edit_file = mock_edit::file_context();
        mock_edit_file
            .expect()
            .with(eq(PathBuf::from("./example_configs/terrain.full.toml")))
            .return_once(|_| Ok(()))
            .times(1);

        let get_central_store_path_context = mock_fs::get_central_store_path_context();
        get_central_store_path_context
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/terrains/")))
            .times(1);

        let terrain = test_data::test_data_terrain_full();
        let main = terrain.get(Some(BiomeArg::None))?;
        let generate_and_compile_context = mock_ops::generate_and_compile_context();
        generate_and_compile_context
            .expect()
            .with(
                eq(PathBuf::from("~/.config/terrainium/terrains/")),
                eq(String::from("none")),
                eq(main),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        let example_biome = terrain.get(Some(BiomeArg::Value("example_biome".to_owned())))?;
        generate_and_compile_context
            .expect()
            .with(
                eq(PathBuf::from("~/.config/terrainium/terrains/")),
                eq(String::from("example_biome")),
                eq(example_biome),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        let example_biome2 = terrain.get(Some(BiomeArg::Value("example_biome2".to_owned())))?;
        generate_and_compile_context
            .expect()
            .with(
                eq(PathBuf::from("~/.config/terrainium/terrains/")),
                eq(String::from("example_biome2")),
                eq(example_biome2),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        handle_edit()?;

        return Ok(());
    }
}
