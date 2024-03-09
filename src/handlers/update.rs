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
        types::{
            args::{BiomeArg, Pair},
            terrain::test_data,
        },
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

    #[test]
    #[serial]
    fn handle_updates_terrain_and_creates_backup() -> Result<()> {
        let test_dir = tempdir()?;
        let test_dir_path: PathBuf = test_dir.path().into();
        let mock_cwd = mock_fs::get_cwd_context();
        mock_cwd
            .expect()
            .returning(move || {
                let test_dir_path: PathBuf = test_dir_path.clone();
                Ok(test_dir_path)
            })
            .times(7);

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

        let mut backup: PathBuf = test_dir.path().into();
        backup.push("terrain.toml.bkp");
        std::fs::copy("./tests/data/terrain.full.toml", &backup)?;

        super::handle(
            Some("example_biome2".to_string()),
            crate::types::args::UpdateOpts {
                new: None,
                biome: None,
                envs: None,
                aliases: None,
            },
            true,
        )?;

        let expected = std::fs::read_to_string("./tests/data/terrain.full.changed.default.toml")?;
        let actual = std::fs::read_to_string(terrain_toml_path)?;

        assert_eq!(expected, actual);

        let expected_bkp = std::fs::read_to_string("./tests/data/terrain.full.toml")?;
        let actual_bkp = std::fs::read_to_string(backup)?;

        assert_eq!(expected_bkp, actual_bkp);

        Ok(())
    }

    #[test]
    #[serial]
    fn handle_updates_specified_biome() -> Result<()> {
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

        let terrain = test_data::terrain_full_updated_example_biome2();
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

        let envs = vec![
            Pair {
                key: "NEW_VAR".to_string(),
                value: "NEW_VALUE".to_string(),
            },
            Pair {
                key: "EDITOR".to_string(),
                value: "UPDATED".to_string(),
            },
        ];

        let aliases = vec![Pair {
            key: "new_alias".to_string(),
            value: "new_value".to_string(),
        }];

        super::handle(
            None,
            crate::types::args::UpdateOpts {
                new: None,
                biome: Some(BiomeArg::Value("example_biome2".to_string())),
                envs: Some(envs),
                aliases: Some(aliases),
            },
            false,
        )?;

        let expected =
            std::fs::read_to_string("./tests/data/terrain.full.updated.example_biome2.toml")?;
        let actual = std::fs::read_to_string(terrain_toml_path)?;

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    #[serial]
    fn handle_creates_new_biome() -> Result<()> {
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

        let terrain = test_data::terrain_full_with_new();
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

        let example_biome3 = terrain.get(Some(BiomeArg::Value("example_biome3".to_owned())))?;
        generate_and_compile_context
            .expect()
            .with(
                eq(scripts_dir_path.clone()),
                eq(String::from("example_biome3")),
                eq(example_biome3),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        let mut terrain_toml_path: PathBuf = test_dir.path().into();
        terrain_toml_path.push("terrain.toml");
        std::fs::copy("./tests/data/terrain.full.toml", &terrain_toml_path)?;

        let envs = vec![Pair {
            key: "EDITOR".to_string(),
            value: "pico".to_string(),
        }];

        let aliases = vec![Pair {
            key: "tenter".to_string(),
            value: "terrainium enter --biome example_biome3".to_string(),
        }];

        super::handle(
            None,
            crate::types::args::UpdateOpts {
                new: Some("example_biome3".to_string()),
                biome: None,
                envs: Some(envs),
                aliases: Some(aliases),
            },
            false,
        )?;

        let expected =
            std::fs::read_to_string("./tests/data/terrain.full.new.example_biome3.toml")?;
        let actual = std::fs::read_to_string(terrain_toml_path)?;

        assert_eq!(expected, actual);

        Ok(())
    }
}
