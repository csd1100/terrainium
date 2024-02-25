use anyhow::{Context, Result};

use crate::{
    helpers::operations::{copy_file, get_current_dir_toml, get_parsed_terrain, write_terrain},
    types::args::UpdateOpts,
};

use super::generate::generate_and_compile_all;

pub fn handle(set_default_biome: Option<String>, opts: UpdateOpts, backup: bool) -> Result<()> {
    let UpdateOpts {
        new,
        biome,
        envs,
        aliases,
    } = opts;

    if backup {
        backup_terrain()?;
    }

    let mut terrain = get_parsed_terrain()?;

    if let Some(new_default) = set_default_biome {
        terrain.set_default_biome(new_default)?;
    } else if let Some(biome_name) = new {
        terrain.create_new_biome(biome_name, envs, aliases)?;
    } else {
        terrain
            .update(biome, envs, aliases)
            .context("failed to update biome")?;
    }

    write_terrain(
        get_current_dir_toml()
            .context("unable to get terrain.toml path")?
            .as_path(),
        &terrain,
    )
    .context("failed to write updated terrain.toml")?;

    generate_and_compile_all(terrain)?;

    Ok(())
}

fn backup_terrain() -> Result<()> {
    let terrain_toml = get_current_dir_toml()?;
    let backup = terrain_toml.with_extension("toml.bkp");
    copy_file(&terrain_toml, &backup).context("unable to backup terrain.toml")?;
    Ok(())
}

#[cfg(test)]
mod test {
    use std::path::{Path, PathBuf};

    use anyhow::Result;
    use mockall::predicate::eq;
    use serial_test::serial;
    use tempfile::tempdir;

    use crate::{
        helpers::utils::mock_fs,
        shell::zsh::mock_ops,
        types::{args::BiomeArg, terrain::test_data},
    };

    #[test]
    #[serial]
    fn handle_only_sets_default_biome() -> Result<()> {
        let test_dir = tempdir()?;
        let test_dir_path: PathBuf = test_dir.path().into();
        let mock_cwd = mock_fs::get_cwd_context();
        mock_cwd
            .expect()
            .returning(move || {
                let test_dir_path: PathBuf = test_dir_path.clone();
                Ok(test_dir_path)
            })
            .times(5);

        let home_dir = tempdir()?;
        let home_dir_path: PathBuf = home_dir.path().into();
        let mock_home = mock_fs::get_home_dir_context();
        mock_home
            .expect()
            .returning(move || {
                let home_dir_path: PathBuf = home_dir_path.clone();
                Ok(home_dir_path)
            })
            .times(1);

        let home_dir_path: PathBuf = home_dir.path().into();
        let test_dir_path: PathBuf = test_dir.path().into();
        let scripts_dir_name = Path::canonicalize(test_dir_path.as_path())?
            .to_string_lossy()
            .to_string()
            .replace('/', "_");
        let scripts_dir_path = home_dir_path.join(PathBuf::from(
            ".config/terrainium/terrains/".to_owned() + &scripts_dir_name,
        ));
        let terrain = test_data::terrain_full();
        let main = terrain.get(Some(BiomeArg::None))?;
        let generate_and_compile_context = mock_ops::generate_and_compile_context();
        generate_and_compile_context
            .expect()
            .with(
                eq(scripts_dir_path.clone()),
                eq(String::from("none")),
                eq(main),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        let example_biome = terrain.get(Some(BiomeArg::Value("example_biome".to_owned())))?;
        generate_and_compile_context
            .expect()
            .with(
                eq(scripts_dir_path.clone()),
                eq(String::from("example_biome")),
                eq(example_biome),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        let example_biome2 = terrain.get(Some(BiomeArg::Value("example_biome2".to_owned())))?;
        generate_and_compile_context
            .expect()
            .with(
                eq(scripts_dir_path.clone()),
                eq(String::from("example_biome2")),
                eq(example_biome2),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        let mut terrain_toml_path: PathBuf = test_dir.path().into();
        terrain_toml_path.push("terrain.toml");
        std::fs::copy("./tests/data/terrain.full.toml", &terrain_toml_path)?;

        super::handle(
            Some("example_biome2".to_string()),
            crate::types::args::UpdateOpts {
                new: None,
                biome: None,
                envs: None,
                aliases: None,
            },
            false,
        )?;

        let expected = std::fs::read_to_string("./tests/data/terrain.full.changed.default.toml")?;
        let actual = std::fs::read_to_string(terrain_toml_path)?;

        assert_eq!(expected, actual);

        Ok(())
    }

    // #[test]
    // #[serial]
    // fn handle_updates_terrain_and_creates_backup() -> Result<()> {
    //     let mock_terrain = mock_fs::get_parsed_terrain_context();
    //     mock_terrain
    //         .expect()
    //         .return_once(|| Ok(test_data::terrain_full()))
    //         .times(1);
    //
    //     let mock_terrain_path = mock_fs::get_current_dir_toml_context();
    //     mock_terrain_path
    //         .expect()
    //         .return_once(|| Ok(PathBuf::from("./example_configs/terrain.full.toml")))
    //         .times(1);
    //
    //     let mock_copy = mock_fs::copy_file_context();
    //     mock_copy
    //         .expect()
    //         .with(
    //             eq(PathBuf::from("./example_configs/terrain.full.toml")),
    //             eq(PathBuf::from("./example_configs/terrain.full.toml.bkp")),
    //         )
    //         .return_once(|_, _| Ok(0))
    //         .times(1);
    //
    //     let env_vars = vec![Pair {
    //         key: "EDITOR".to_string(),
    //         value: "nano".to_string(),
    //     }];
    //     let aliases = vec![Pair {
    //         key: "new_test".to_string(),
    //         value: "new_value".to_string(),
    //     }];
    //     let mut expected = test_data::terrain_full();
    //     expected.update(Some(BiomeArg::Default), Some(env_vars), Some(aliases))?;
    //
    //     mock_terrain_path
    //         .expect()
    //         .return_once(|| Ok(PathBuf::from("./example_configs/terrain.full.toml")))
    //         .times(1);
    //     let mock_write_terrain = mock_fs::write_terrain_context();
    //     mock_write_terrain
    //         .expect()
    //         .with(
    //             eq(PathBuf::from("./example_configs/terrain.full.toml")),
    //             eq(expected),
    //         )
    //         .times(1)
    //         .return_once(|_, _| Ok(()));
    //
    //     let get_central_store_path_context = mock_fs::get_central_store_path_context();
    //     get_central_store_path_context
    //         .expect()
    //         .return_once(|| Ok(PathBuf::from("~/.config/terrainium/terrains/")))
    //         .times(1);
    //
    //     let remove_all_script_files = mock_fs::remove_all_script_files_context();
    //     remove_all_script_files
    //         .expect()
    //         .withf(|path| path == PathBuf::from("~/.config/terrainium/terrains/").as_path())
    //         .return_once(|_| Ok(()))
    //         .times(1);
    //
    //     let terrain = test_data::terrain_full();
    //     let main = terrain.get(Some(BiomeArg::None))?;
    //     let generate_and_compile_context = mock_ops::generate_and_compile_context();
    //     generate_and_compile_context
    //         .expect()
    //         .with(
    //             eq(PathBuf::from("~/.config/terrainium/terrains/")),
    //             eq(String::from("none")),
    //             eq(main),
    //         )
    //         .return_once(|_, _, _| Ok(()))
    //         .times(1);
    //
    //     let mut example_biome = terrain.get(Some(BiomeArg::Value("example_biome".to_owned())))?;
    //     example_biome.update_env("EDITOR".to_string(), "nano".to_string());
    //     example_biome.update_alias("new_test".to_string(), "new_value".to_string());
    //     generate_and_compile_context
    //         .expect()
    //         .with(
    //             eq(PathBuf::from("~/.config/terrainium/terrains/")),
    //             eq(String::from("example_biome")),
    //             eq(example_biome),
    //         )
    //         .return_once(|_, _, _| Ok(()))
    //         .times(1);
    //
    //     let example_biome2 = terrain.get(Some(BiomeArg::Value("example_biome2".to_owned())))?;
    //     generate_and_compile_context
    //         .expect()
    //         .with(
    //             eq(PathBuf::from("~/.config/terrainium/terrains/")),
    //             eq(String::from("example_biome2")),
    //             eq(example_biome2),
    //         )
    //         .return_once(|_, _, _| Ok(()))
    //         .times(1);
    //
    //     let env_vars = vec![Pair {
    //         key: "EDITOR".to_string(),
    //         value: "nano".to_string(),
    //     }];
    //
    //     let aliases = vec![Pair {
    //         key: "new_test".to_string(),
    //         value: "new_value".to_string(),
    //     }];
    //
    //     super::handle(
    //         None,
    //         UpdateOpts {
    //             new: None,
    //             biome: None,
    //             envs: Some(env_vars),
    //             aliases: Some(aliases),
    //         },
    //         true,
    //     )?;
    //
    //     Ok(())
    // }
    //
    //     #[test]
    //     #[serial]
    //     fn handle_updates_specified_biome() -> Result<()> {
    //         let mock_terrain = mock_fs::get_parsed_terrain_context();
    //         mock_terrain
    //             .expect()
    //             .return_once(|| Ok(test_data::terrain_full()))
    //             .times(1);
    //
    //         let mock_terrain_path = mock_fs::get_current_dir_toml_context();
    //         mock_terrain_path
    //             .expect()
    //             .return_once(|| Ok(PathBuf::from("./example_configs/terrain.full.toml")))
    //             .times(1);
    //
    //         let env_vars = vec![Pair {
    //             key: "EDITOR".to_string(),
    //             value: "nano".to_string(),
    //         }];
    //         let aliases = vec![Pair {
    //             key: "new_test".to_string(),
    //             value: "new_value".to_string(),
    //         }];
    //
    //         let mut expected = test_data::terrain_full();
    //         expected.update(
    //             Some(BiomeArg::Value("example_biome2".to_string())),
    //             Some(env_vars),
    //             Some(aliases),
    //         )?;
    //
    //         let mock_write_terrain = mock_fs::write_terrain_context();
    //         mock_write_terrain
    //             .expect()
    //             .with(
    //                 eq(PathBuf::from("./example_configs/terrain.full.toml")),
    //                 eq(expected),
    //             )
    //             .times(1)
    //             .return_once(|_, _| Ok(()));
    //
    //         let get_central_store_path_context = mock_fs::get_central_store_path_context();
    //         get_central_store_path_context
    //             .expect()
    //             .return_once(|| Ok(PathBuf::from("~/.config/terrainium/terrains/")))
    //             .times(1);
    //
    //         let remove_all_script_files = mock_fs::remove_all_script_files_context();
    //         remove_all_script_files
    //             .expect()
    //             .withf(|path| path == PathBuf::from("~/.config/terrainium/terrains/").as_path())
    //             .return_once(|_| Ok(()))
    //             .times(1);
    //
    //         let terrain = test_data::terrain_full();
    //         let main = terrain.get(Some(BiomeArg::None))?;
    //         let generate_and_compile_context = mock_ops::generate_and_compile_context();
    //         generate_and_compile_context
    //             .expect()
    //             .with(
    //                 eq(PathBuf::from("~/.config/terrainium/terrains/")),
    //                 eq(String::from("none")),
    //                 eq(main),
    //             )
    //             .return_once(|_, _, _| Ok(()))
    //             .times(1);
    //
    //         let example_biome = terrain.get(Some(BiomeArg::Value("example_biome".to_owned())))?;
    //         generate_and_compile_context
    //             .expect()
    //             .with(
    //                 eq(PathBuf::from("~/.config/terrainium/terrains/")),
    //                 eq(String::from("example_biome")),
    //                 eq(example_biome),
    //             )
    //             .return_once(|_, _, _| Ok(()))
    //             .times(1);
    //
    //         let mut example_biome2 = terrain.get(Some(BiomeArg::Value("example_biome2".to_owned())))?;
    //         example_biome2.update_env("EDITOR".to_string(), "nano".to_string());
    //         example_biome2.update_alias("new_test".to_string(), "new_value".to_string());
    //         generate_and_compile_context
    //             .expect()
    //             .with(
    //                 eq(PathBuf::from("~/.config/terrainium/terrains/")),
    //                 eq(String::from("example_biome2")),
    //                 eq(example_biome2),
    //             )
    //             .return_once(|_, _, _| Ok(()))
    //             .times(1);
    //
    //         let env_vars = vec![Pair {
    //             key: "EDITOR".to_string(),
    //             value: "nano".to_string(),
    //         }];
    //
    //         let aliases = vec![Pair {
    //             key: "new_test".to_string(),
    //             value: "new_value".to_string(),
    //         }];
    //
    //         super::handle(
    //             None,
    //             UpdateOpts {
    //                 new: None,
    //                 biome: Some(BiomeArg::Value("example_biome2".to_string())),
    //                 envs: Some(env_vars),
    //                 aliases: Some(aliases),
    //             },
    //             false,
    //         )?;
    //
    //         Ok(())
    //     }
    //     #[test]
    //     #[serial]
    //     fn handle_creates_new_biome() -> Result<()> {
    //         let mock_terrain = mock_fs::get_parsed_terrain_context();
    //         mock_terrain
    //             .expect()
    //             .return_once(|| Ok(test_data::terrain_full()))
    //             .times(1);
    //
    //         let mock_terrain_path = mock_fs::get_current_dir_toml_context();
    //         mock_terrain_path
    //             .expect()
    //             .return_once(|| Ok(PathBuf::from("./example_configs/terrain.full.toml")))
    //             .times(1);
    //
    //         let env_vars = vec![Pair {
    //             key: "EDITOR".to_string(),
    //             value: "nano".to_string(),
    //         }];
    //
    //         let aliases = vec![Pair {
    //             key: "new_test".to_string(),
    //             value: "new_value".to_string(),
    //         }];
    //
    //         let mut expected = test_data::terrain_full();
    //         expected.add_biome(&"new".to_string(), Biome::new())?;
    //         expected.update(
    //             Some(BiomeArg::Value("new".to_string())),
    //             Some(env_vars.clone()),
    //             Some(aliases.clone()),
    //         )?;
    //
    //         let mock_write_terrain = mock_fs::write_terrain_context();
    //         mock_write_terrain
    //             .expect()
    //             .with(
    //                 eq(PathBuf::from("./example_configs/terrain.full.toml")),
    //                 eq(expected),
    //             )
    //             .times(1)
    //             .return_once(|_, _| Ok(()));
    //
    //         let get_central_store_path_context = mock_fs::get_central_store_path_context();
    //         get_central_store_path_context
    //             .expect()
    //             .return_once(|| Ok(PathBuf::from("~/.config/terrainium/terrains/")))
    //             .times(1);
    //
    //         let remove_all_script_files = mock_fs::remove_all_script_files_context();
    //         remove_all_script_files
    //             .expect()
    //             .withf(|path| path == PathBuf::from("~/.config/terrainium/terrains/").as_path())
    //             .return_once(|_| Ok(()))
    //             .times(1);
    //
    //         let generate_and_compile_context = mock_ops::generate_and_compile_context();
    //
    //         let mut expected = test_data::terrain_full();
    //         expected.add_biome(&"new".to_string(), Biome::new())?;
    //         expected.update(
    //             Some(BiomeArg::Value("new".to_string())),
    //             Some(env_vars),
    //             Some(aliases),
    //         )?;
    //
    //         let new = expected.get(Some(BiomeArg::Value("new".to_owned())))?;
    //         generate_and_compile_context
    //             .expect()
    //             .with(
    //                 eq(PathBuf::from("~/.config/terrainium/terrains/")),
    //                 eq(String::from("new")),
    //                 eq(new),
    //             )
    //             .return_once(|_, _, _| Ok(()))
    //             .times(1);
    //
    //         let main = expected.get(Some(BiomeArg::None))?;
    //         generate_and_compile_context
    //             .expect()
    //             .with(
    //                 eq(PathBuf::from("~/.config/terrainium/terrains/")),
    //                 eq(String::from("none")),
    //                 eq(main),
    //             )
    //             .return_once(|_, _, _| Ok(()))
    //             .times(1);
    //
    //         let example_biome = expected.get(Some(BiomeArg::Value("example_biome".to_owned())))?;
    //         generate_and_compile_context
    //             .expect()
    //             .with(
    //                 eq(PathBuf::from("~/.config/terrainium/terrains/")),
    //                 eq(String::from("example_biome")),
    //                 eq(example_biome),
    //             )
    //             .return_once(|_, _, _| Ok(()))
    //             .times(1);
    //
    //         let example_biome2 = expected.get(Some(BiomeArg::Value("example_biome2".to_owned())))?;
    //         generate_and_compile_context
    //             .expect()
    //             .with(
    //                 eq(PathBuf::from("~/.config/terrainium/terrains/")),
    //                 eq(String::from("example_biome2")),
    //                 eq(example_biome2),
    //             )
    //             .return_once(|_, _, _| Ok(()))
    //             .times(1);
    //
    //         let env_vars = vec![Pair {
    //             key: "EDITOR".to_string(),
    //             value: "nano".to_string(),
    //         }];
    //
    //         let aliases = vec![Pair {
    //             key: "new_test".to_string(),
    //             value: "new_value".to_string(),
    //         }];
    //
    //         super::handle(
    //             None,
    //             UpdateOpts {
    //                 new: Some("new".to_string()),
    //                 biome: None,
    //                 envs: Some(env_vars),
    //                 aliases: Some(aliases),
    //             },
    //             false,
    //         )?;
    //
    //         Ok(())
    //     }
}
