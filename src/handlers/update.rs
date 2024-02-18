use anyhow::{anyhow, Context, Result};
use mockall_double::double;

use crate::types::{
    args::{BiomeArg, UpdateOpts},
    biomes::Biome,
    terrain::parse_terrain,
};

#[double]
use crate::shell::zsh::ops;

#[double]
use crate::helpers::operations::fs;

pub fn handle(set_biome: Option<String>, opts: UpdateOpts, backup: bool) -> Result<()> {
    let UpdateOpts {
        new,
        biome,
        env,
        alias,
    } = opts;

    let toml_file = fs::get_current_dir_toml().context("unable to get terrain.toml path")?;

    if backup {
        let bkp = toml_file.with_extension("toml.bkp");
        fs::copy_file(&toml_file, &bkp).context("unable to backup terrain.toml")?;
    }

    let mut terrain = parse_terrain(&toml_file)?;

    if let Some(default) = set_biome {
        terrain
            .update_default_biome(default)
            .context("unable to update default biome")?;
    } else if let Some(biome) = &new {
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
    }

    fs::write_terrain(toml_file.as_path(), &terrain)
        .context("failed to write updated terrain.toml")?;

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

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use anyhow::Result;
    use mockall::predicate::eq;
    use serial_test::serial;

    use crate::{
        helpers::operations::mock_fs,
        shell::zsh::mock_ops,
        types::{
            args::{BiomeArg, Pair, UpdateOpts},
            biomes::Biome,
            terrain::test_data,
        },
    };

    #[test]
    #[serial]
    fn handle_only_sets_default_biome() -> Result<()> {
        let mock_terrain_path = mock_fs::get_current_dir_toml_context();
        mock_terrain_path
            .expect()
            .return_once(|| Ok(PathBuf::from("./example_configs/terrain.full.toml")))
            .times(1);

        let mut expected = test_data::terrain_full();
        expected.update_default_biome("example_biome2".to_string())?;

        let mock_write_terrain = mock_fs::write_terrain_context();
        mock_write_terrain
            .expect()
            .with(
                eq(PathBuf::from("./example_configs/terrain.full.toml")),
                eq(expected),
            )
            .times(1)
            .return_once(|_, _| Ok(()));

        let get_central_store_path_context = mock_fs::get_central_store_path_context();
        get_central_store_path_context
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/terrains/")))
            .times(1);

        let terrain = test_data::terrain_full();
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

        let env_vars = vec![Pair {
            key: "EDITOR".to_string(),
            value: "nano".to_string(),
        }];

        super::handle(
            Some("example_biome2".to_string()),
            UpdateOpts {
                new: None,
                biome: None,
                env: Some(env_vars),
                alias: None,
            },
            false,
        )?;

        Ok(())
    }

    #[test]
    #[serial]
    fn handle_updates_terrain_and_creates_backup() -> Result<()> {
        let mock_terrain_path = mock_fs::get_current_dir_toml_context();
        mock_terrain_path
            .expect()
            .return_once(|| Ok(PathBuf::from("./example_configs/terrain.full.toml")))
            .times(1);

        let mock_copy = mock_fs::copy_file_context();
        mock_copy
            .expect()
            .with(
                eq(PathBuf::from("./example_configs/terrain.full.toml")),
                eq(PathBuf::from("./example_configs/terrain.full.toml.bkp")),
            )
            .return_once(|_, _| Ok(0))
            .times(1);

        let env_vars = vec![Pair {
            key: "EDITOR".to_string(),
            value: "nano".to_string(),
        }];
        let aliases = vec![Pair {
            key: "new_test".to_string(),
            value: "new_value".to_string(),
        }];

        let mut expected = test_data::terrain_full();
        expected.update(Some(BiomeArg::Default), Some(env_vars), Some(aliases))?;

        let mock_write_terrain = mock_fs::write_terrain_context();
        mock_write_terrain
            .expect()
            .with(
                eq(PathBuf::from("./example_configs/terrain.full.toml")),
                eq(expected),
            )
            .times(1)
            .return_once(|_, _| Ok(()));

        let get_central_store_path_context = mock_fs::get_central_store_path_context();
        get_central_store_path_context
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/terrains/")))
            .times(1);

        let terrain = test_data::terrain_full();
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

        let mut example_biome = terrain.get(Some(BiomeArg::Value("example_biome".to_owned())))?;
        example_biome.update_env("EDITOR".to_string(), "nano".to_string());
        example_biome.update_alias("new_test".to_string(), "new_value".to_string());
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

        let env_vars = vec![Pair {
            key: "EDITOR".to_string(),
            value: "nano".to_string(),
        }];

        let aliases = vec![Pair {
            key: "new_test".to_string(),
            value: "new_value".to_string(),
        }];

        super::handle(
            None,
            UpdateOpts {
                new: None,
                biome: None,
                env: Some(env_vars),
                alias: Some(aliases),
            },
            true,
        )?;

        Ok(())
    }

    #[test]
    #[serial]
    fn handle_updates_specified_biome() -> Result<()> {
        let mock_terrain_path = mock_fs::get_current_dir_toml_context();
        mock_terrain_path
            .expect()
            .return_once(|| Ok(PathBuf::from("./example_configs/terrain.full.toml")))
            .times(1);

        let env_vars = vec![Pair {
            key: "EDITOR".to_string(),
            value: "nano".to_string(),
        }];
        let aliases = vec![Pair {
            key: "new_test".to_string(),
            value: "new_value".to_string(),
        }];

        let mut expected = test_data::terrain_full();
        expected.update(
            Some(BiomeArg::Value("example_biome2".to_string())),
            Some(env_vars),
            Some(aliases),
        )?;

        let mock_write_terrain = mock_fs::write_terrain_context();
        mock_write_terrain
            .expect()
            .with(
                eq(PathBuf::from("./example_configs/terrain.full.toml")),
                eq(expected),
            )
            .times(1)
            .return_once(|_, _| Ok(()));

        let get_central_store_path_context = mock_fs::get_central_store_path_context();
        get_central_store_path_context
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/terrains/")))
            .times(1);

        let terrain = test_data::terrain_full();
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

        let mut example_biome2 = terrain.get(Some(BiomeArg::Value("example_biome2".to_owned())))?;
        example_biome2.update_env("EDITOR".to_string(), "nano".to_string());
        example_biome2.update_alias("new_test".to_string(), "new_value".to_string());
        generate_and_compile_context
            .expect()
            .with(
                eq(PathBuf::from("~/.config/terrainium/terrains/")),
                eq(String::from("example_biome2")),
                eq(example_biome2),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        let env_vars = vec![Pair {
            key: "EDITOR".to_string(),
            value: "nano".to_string(),
        }];

        let aliases = vec![Pair {
            key: "new_test".to_string(),
            value: "new_value".to_string(),
        }];

        super::handle(
            None,
            UpdateOpts {
                new: None,
                biome: Some(BiomeArg::Value("example_biome2".to_string())),
                env: Some(env_vars),
                alias: Some(aliases),
            },
            false,
        )?;

        Ok(())
    }
    #[test]
    #[serial]
    fn handle_creates_new_biome() -> Result<()> {
        let mock_terrain_path = mock_fs::get_current_dir_toml_context();
        mock_terrain_path
            .expect()
            .return_once(|| Ok(PathBuf::from("./example_configs/terrain.full.toml")))
            .times(1);

        let env_vars = vec![Pair {
            key: "EDITOR".to_string(),
            value: "nano".to_string(),
        }];

        let aliases = vec![Pair {
            key: "new_test".to_string(),
            value: "new_value".to_string(),
        }];

        let mut expected = test_data::terrain_full();
        expected.add_biome(&"new".to_string(), Biome::new())?;
        expected.update(
            Some(BiomeArg::Value("new".to_string())),
            Some(env_vars.clone()),
            Some(aliases.clone()),
        )?;

        let mock_write_terrain = mock_fs::write_terrain_context();
        mock_write_terrain
            .expect()
            .with(
                eq(PathBuf::from("./example_configs/terrain.full.toml")),
                eq(expected),
            )
            .times(1)
            .return_once(|_, _| Ok(()));

        let get_central_store_path_context = mock_fs::get_central_store_path_context();
        get_central_store_path_context
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/terrains/")))
            .times(1);

        let generate_and_compile_context = mock_ops::generate_and_compile_context();

        let mut expected = test_data::terrain_full();
        expected.add_biome(&"new".to_string(), Biome::new())?;
        expected.update(
            Some(BiomeArg::Value("new".to_string())),
            Some(env_vars),
            Some(aliases),
        )?;

        let new = expected.get(Some(BiomeArg::Value("new".to_owned())))?;
        generate_and_compile_context
            .expect()
            .with(
                eq(PathBuf::from("~/.config/terrainium/terrains/")),
                eq(String::from("new")),
                eq(new),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        let main = expected.get(Some(BiomeArg::None))?;
        generate_and_compile_context
            .expect()
            .with(
                eq(PathBuf::from("~/.config/terrainium/terrains/")),
                eq(String::from("none")),
                eq(main),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        let example_biome = expected.get(Some(BiomeArg::Value("example_biome".to_owned())))?;
        generate_and_compile_context
            .expect()
            .with(
                eq(PathBuf::from("~/.config/terrainium/terrains/")),
                eq(String::from("example_biome")),
                eq(example_biome),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        let example_biome2 = expected.get(Some(BiomeArg::Value("example_biome2".to_owned())))?;
        generate_and_compile_context
            .expect()
            .with(
                eq(PathBuf::from("~/.config/terrainium/terrains/")),
                eq(String::from("example_biome2")),
                eq(example_biome2),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        let env_vars = vec![Pair {
            key: "EDITOR".to_string(),
            value: "nano".to_string(),
        }];

        let aliases = vec![Pair {
            key: "new_test".to_string(),
            value: "new_value".to_string(),
        }];

        super::handle(
            None,
            UpdateOpts {
                new: Some("new".to_string()),
                biome: None,
                env: Some(env_vars),
                alias: Some(aliases),
            },
            false,
        )?;

        Ok(())
    }
}
