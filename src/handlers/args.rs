use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use mockall_double::double;
use uuid::Uuid;

use crate::{
    shell::{
        background::start_background_processes,
        zsh::{get_zsh_envs, spawn_zsh},
    },
    types::{
        args::{BiomeArg, GetOpts},
        terrain::parse_terrain,
    },
};

use super::{
    constants::{TERRAINIUM_ENABLED, TERRAINIUM_SESSION_ID},
    helpers::merge_hashmaps,
};

#[double]
use super::helpers::fs;

#[double]
use crate::shell::editor::edit;

#[double]
use crate::shell::zsh::ops;

#[double]
use crate::templates::get::print;

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

    Ok(())
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

    Ok(())
}

pub fn handle_get(all: bool, biome: Option<BiomeArg>, opts: GetOpts) -> Result<()> {
    let terrain = fs::get_parsed_terrain()?;
    if opts.is_empty() || all {
        let mut terrain = terrain
            .get_printable_terrain(biome)
            .context("failed to get printable terrain")?;
        terrain.all = true;
        print::all(terrain)?;
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
            print::aliases(terrain.alias.to_owned()).context("unable to print aliases")?;
        } else if let Some(alias) = alias {
            let found_alias = Some(
                terrain
                    .find_aliases(alias)
                    .context("unable to get aliases")?,
            );
            let aliases: HashMap<String, String> = found_alias
                .expect("to be present")
                .iter()
                .map(|(k, v)| {
                    if v.is_none() {
                        (k.to_string(), "NOT FOUND".to_string())
                    } else {
                        (
                            k.to_string(),
                            v.to_owned().expect("to be present").to_string(),
                        )
                    }
                })
                .collect();
            print::aliases(Some(aliases)).context("unable to print aliases")?;
        }

        if env_all {
            print::env(terrain.env.to_owned()).context("unable to print env vars")?;
        } else if let Some(env) = env {
            let found_env = Some(terrain.find_envs(env).context("unable to get env vars")?);
            let env: HashMap<String, String> = found_env
                .expect("to be present")
                .iter()
                .map(|(k, v)| {
                    if v.is_none() {
                        (k.to_string(), "NOT FOUND".to_string())
                    } else {
                        (
                            k.to_string(),
                            v.to_owned().expect("to be present").to_string(),
                        )
                    }
                })
                .collect();
            print::env(Some(env)).context("unable to print env vars")?;
        }

        if constructors {
            print::constructors(terrain.constructors).context("unable to print constructors")?;
        }
        if destructors {
            print::destructors(terrain.destructors).context("unable to print destructors")?;
        }
    }
    Ok(())
}

pub fn handle_enter(biome: Option<BiomeArg>) -> Result<()> {
    let terrain = fs::get_parsed_terrain()?;
    let selected = terrain
        .get(biome.clone())
        .context("unable to select biome")?;
    let mut envs = selected.env;

    if envs.is_none() {
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

    Ok(())
}

pub fn handle_exit(biome: Option<BiomeArg>) -> Result<()> {
    handle_deconstruct(biome).context("unable to call destructors")
}

pub fn handle_construct(
    biome: Option<BiomeArg>,
    envs: Option<&HashMap<String, String>>,
) -> Result<()> {
    let terrain = fs::get_parsed_terrain()?
        .get(biome)
        .context("unable to select biome to call constructors")?;
    if let Some(envs) = envs {
        return start_background_processes(terrain.constructors, envs)
            .context("unable to start background processes");
    }
    start_background_processes(terrain.constructors, &terrain.env.unwrap_or_default())
        .context("unable to start background processes")
}

pub fn handle_deconstruct(biome: Option<BiomeArg>) -> Result<()> {
    let terrain = fs::get_parsed_terrain()?
        .get(biome)
        .context("unable to select biome to call destructors")?;
    start_background_processes(terrain.destructors, &terrain.env.unwrap_or_default())
        .context("unable to start background processes")
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, path::PathBuf};

    use anyhow::Result;
    use mockall::predicate::eq;
    use serial_test::serial;

    use crate::{
        handlers::helpers::mock_fs,
        shell::{editor::mock_edit, zsh::mock_ops},
        templates::get::mock_print,
        types::{
            args::{BiomeArg, GetOpts},
            commands::{Command, Commands},
            terrain::test_data::{self, test_data_terrain_full},
        },
    };

    use super::{handle_edit, handle_generate, handle_get};

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

        Ok(())
    }

    #[test]
    #[serial]
    fn handle_generate_generates_scripts() -> Result<()> {
        let mock_get_toml_path = mock_fs::get_terrain_toml_context();
        mock_get_toml_path
            .expect()
            .return_once(|| Ok(PathBuf::from("./example_configs/terrain.full.toml")))
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

        handle_generate()?;

        Ok(())
    }

    #[test]
    #[serial]
    fn handle_get_all_calls_print_all() -> Result<()> {
        let mock_get_parsed_terrain = mock_fs::get_parsed_terrain_context();
        mock_get_parsed_terrain
            .expect()
            .return_once(|| Ok(test_data::test_data_terrain_full()))
            .times(1);

        let mock_print_all = mock_print::all_context();

        let mut printable = test_data_terrain_full().get_printable_terrain(None)?;
        printable.all = true;

        mock_print_all
            .expect()
            .with(eq(printable))
            .return_once(|_| Ok(()));

        let opts = GetOpts {
            alias_all: false,
            alias: None,
            env_all: false,
            env: None,
            constructors: false,
            destructors: false,
        };
        handle_get(true, None, opts)?;
        return Ok(());
    }

    #[test]
    #[serial]
    fn handle_get_empty_opts_calls_print_all() -> Result<()> {
        let mock_get_parsed_terrain = mock_fs::get_parsed_terrain_context();
        mock_get_parsed_terrain
            .expect()
            .return_once(|| Ok(test_data::test_data_terrain_full()))
            .times(1);

        let mock_print_all = mock_print::all_context();

        let mut printable = test_data_terrain_full().get_printable_terrain(None)?;
        printable.all = true;

        mock_print_all
            .expect()
            .with(eq(printable))
            .return_once(|_| Ok(()));

        let opts = GetOpts {
            alias_all: false,
            alias: None,
            env_all: false,
            env: None,
            constructors: false,
            destructors: false,
        };

        handle_get(false, None, opts)?;
        return Ok(());
    }

    #[test]
    #[serial]
    fn handle_get_env_all_calls_print_env_with_all() -> Result<()> {
        let mock_get_parsed_terrain = mock_fs::get_parsed_terrain_context();
        mock_get_parsed_terrain
            .expect()
            .return_once(|| Ok(test_data::test_data_terrain_full()))
            .times(1);

        let mock_print_env = mock_print::env_context();

        let mut envs = HashMap::<String, String>::new();
        envs.insert("EDITOR".to_string(), "nvim".to_string());
        envs.insert("TEST".to_string(), "value".to_string());

        mock_print_env
            .expect()
            .with(eq(Some(envs)))
            .return_once(|_| Ok(()));

        let opts = GetOpts {
            alias_all: false,
            alias: None,
            env_all: true,
            env: None,
            constructors: false,
            destructors: false,
        };

        handle_get(false, None, opts)?;
        return Ok(());
    }

    #[test]
    #[serial]
    fn handle_get_env_calls_print_env() -> Result<()> {
        let mock_get_parsed_terrain = mock_fs::get_parsed_terrain_context();
        mock_get_parsed_terrain
            .expect()
            .return_once(|| Ok(test_data::test_data_terrain_full()))
            .times(1);

        let mock_print_env = mock_print::env_context();

        let mut envs = HashMap::<String, String>::new();
        envs.insert("EDITOR".to_string(), "nvim".to_string());
        envs.insert("NONEXISTENT".to_string(), "NOT FOUND".to_string());

        mock_print_env
            .expect()
            .with(eq(Some(envs)))
            .return_once(|_| Ok(()));

        let opts = GetOpts {
            alias_all: false,
            alias: None,
            env_all: false,
            env: Some(vec!["EDITOR".to_string(), "NONEXISTENT".to_string()]),
            constructors: false,
            destructors: false,
        };

        handle_get(false, None, opts)?;
        return Ok(());
    }

    #[test]
    #[serial]
    fn handle_get_alias_all_calls_print_alias_with_all() -> Result<()> {
        let mock_get_parsed_terrain = mock_fs::get_parsed_terrain_context();
        mock_get_parsed_terrain
            .expect()
            .return_once(|| Ok(test_data::test_data_terrain_full()))
            .times(1);

        let mock_print_aliases = mock_print::aliases_context();

        let mut aliases = HashMap::<String, String>::new();
        aliases.insert("tedit".to_string(), "terrainium edit".to_string());
        aliases.insert(
            "tenter".to_string(),
            "terrainium enter --biome example_biome2".to_string(),
        );

        mock_print_aliases
            .expect()
            .with(eq(Some(aliases)))
            .return_once(|_| Ok(()));

        let opts = GetOpts {
            alias_all: true,
            alias: None,
            env_all: false,
            env: None,
            constructors: false,
            destructors: false,
        };

        handle_get(
            false,
            Some(BiomeArg::Value("example_biome2".to_string())),
            opts,
        )?;
        return Ok(());
    }

    #[test]
    #[serial]
    fn handle_get_alias_calls_print_alias() -> Result<()> {
        let mock_get_parsed_terrain = mock_fs::get_parsed_terrain_context();
        mock_get_parsed_terrain
            .expect()
            .return_once(|| Ok(test_data::test_data_terrain_full()))
            .times(1);

        let mock_print_aliases = mock_print::aliases_context();

        let mut aliases = HashMap::<String, String>::new();
        aliases.insert("NONEXISTENT".to_string(), "NOT FOUND".to_string());
        aliases.insert(
            "tenter".to_string(),
            "terrainium enter --biome example_biome2".to_string(),
        );

        mock_print_aliases
            .expect()
            .with(eq(Some(aliases)))
            .return_once(|_| Ok(()));

        let opts = GetOpts {
            alias_all: false,
            alias: Some(vec!["tenter".to_string(), "NONEXISTENT".to_string()]),
            env_all: false,
            env: None,
            constructors: false,
            destructors: false,
        };

        handle_get(
            false,
            Some(BiomeArg::Value("example_biome2".to_string())),
            opts,
        )?;
        return Ok(());
    }

    #[test]
    #[serial]
    fn handle_get_constructors_calls_print_constructors() -> Result<()> {
        let mock_get_parsed_terrain = mock_fs::get_parsed_terrain_context();
        mock_get_parsed_terrain
            .expect()
            .return_once(|| Ok(test_data::test_data_terrain_full()))
            .times(1);

        let mock_print_constructors = mock_print::constructors_context();

        let constructors = Commands {
            background: None,
            foreground: Some(vec![Command {
                exe: "echo".to_string(),
                args: Some(vec!["entering terrain".to_string()]),
            }]),
        };

        mock_print_constructors
            .expect()
            .with(eq(Some(constructors)))
            .return_once(|_| Ok(()));

        let opts = GetOpts {
            alias_all: false,
            alias: None,
            env_all: false,
            env: None,
            constructors: true,
            destructors: false,
        };

        handle_get(false, Some(BiomeArg::None), opts)?;
        return Ok(());
    }

    #[test]
    #[serial]
    fn handle_get_destructors_calls_print_destructors() -> Result<()> {
        let mock_get_parsed_terrain = mock_fs::get_parsed_terrain_context();
        mock_get_parsed_terrain
            .expect()
            .return_once(|| Ok(test_data::test_data_terrain_full()))
            .times(1);

        let mock_print_destructors = mock_print::destructors_context();

        let destructors = Commands {
            background: None,
            foreground: Some(vec![Command {
                exe: "echo".to_string(),
                args: Some(vec!["exiting terrain".to_string()]),
            }]),
        };

        mock_print_destructors
            .expect()
            .with(eq(Some(destructors)))
            .return_once(|_| Ok(()));

        let opts = GetOpts {
            alias_all: false,
            alias: None,
            env_all: false,
            env: None,
            constructors: false,
            destructors: true,
        };

        handle_get(false, Some(BiomeArg::None), opts)?;
        return Ok(());
    }

    #[test]
    #[serial]
    fn handle_get_mix() -> Result<()> {
        let mock_get_parsed_terrain = mock_fs::get_parsed_terrain_context();
        mock_get_parsed_terrain
            .expect()
            .return_once(|| Ok(test_data::test_data_terrain_full()))
            .times(1);

        let mock_print_env = mock_print::env_context();
        let mut envs = HashMap::<String, String>::new();
        envs.insert("EDITOR".to_string(), "vim".to_string());
        envs.insert("NONEXISTENT".to_string(), "NOT FOUND".to_string());
        mock_print_env
            .expect()
            .with(eq(Some(envs)))
            .return_once(|_| Ok(()));

        let mock_print_aliases = mock_print::aliases_context();
        let mut aliases = HashMap::<String, String>::new();
        aliases.insert("NONEXISTENT".to_string(), "NOT FOUND".to_string());
        aliases.insert(
            "tenter".to_string(),
            "terrainium enter".to_string(),
        );
        mock_print_aliases
            .expect()
            .with(eq(Some(aliases)))
            .return_once(|_| Ok(()));

        let mock_print_constructors = mock_print::constructors_context();
        let constructors = Commands {
            background: None,
            foreground: Some(vec![Command {
                exe: "echo".to_string(),
                args: Some(vec!["entering terrain".to_string()]),
            }]),
        };
        mock_print_constructors
            .expect()
            .with(eq(Some(constructors)))
            .return_once(|_| Ok(()));

        let mock_print_destructors = mock_print::destructors_context();
        let destructors = Commands {
            background: None,
            foreground: Some(vec![Command {
                exe: "echo".to_string(),
                args: Some(vec!["exiting terrain".to_string()]),
            }]),
        };
        mock_print_destructors
            .expect()
            .with(eq(Some(destructors)))
            .return_once(|_| Ok(()));

        let opts = GetOpts {
            alias_all: false,
            alias: Some(vec!["tenter".to_string(), "NONEXISTENT".to_string()]),
            env_all: false,
            env: Some(vec!["EDITOR".to_string(), "NONEXISTENT".to_string()]),
            constructors: true,
            destructors: true,
        };

        handle_get(false, Some(BiomeArg::None), opts)?;
        return Ok(());
    }
}
