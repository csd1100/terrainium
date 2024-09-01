use std::{collections::BTreeMap, str::FromStr};

use anyhow::{anyhow, Context, Result};

#[cfg(test)]
use mockall::automock;

use crate::helpers::utils::Paths;
use crate::{
    helpers::{constants::TERRAINIUM_SELECTED_BIOME, operations::get_terrain_toml_from_biome},
    types::{args::BiomeArg, biomes::Biome, commands::Commands, terrain},
};

pub fn build(get_commands: fn(Biome) -> Option<Commands>, paths: &Paths) -> Result<()> {
    let biome = if let Ok(current) = std::env::var(TERRAINIUM_SELECTED_BIOME) {
        Some(BiomeArg::from_str(&current)?)
    } else {
        return Err(anyhow!("no active biome found"));
    };

    let terrain_toml = get_terrain_toml_from_biome(&biome, paths)?;
    let terrain = terrain::parse_terrain_from(terrain_toml)?
        .get(biome)
        .context("unable to select a biome to call constructors")?;

    let envs = terrain
        .env
        .clone()
        .unwrap_or(BTreeMap::<String, String>::new());

    let commands = get_commands(terrain);

    if let Some(commands) = commands {
        run::commands(commands.background, envs)
            .context("error while starting background processes")?;
    }

    Ok(())
}

#[cfg_attr(test, automock)]
pub mod run {
    use anyhow::Result;
    use mockall_double::double;
    use std::collections::BTreeMap;

    use crate::types::commands::Command;

    #[double]
    use crate::shell::background::processes;

    pub fn commands(
        background: Option<Vec<Command>>,
        envs: BTreeMap<String, String>,
    ) -> Result<()> {
        if let Some(background) = background {
            processes::start(background, envs)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::{collections::BTreeMap, path::PathBuf};

    use crate::helpers::utils::get_paths;
    use crate::{
        shell::background::mock_processes,
        types::{biomes::Biome, commands::Command},
    };
    use anyhow::{anyhow, Result};
    use serial_test::serial;
    use tempfile::tempdir;

    #[test]
    #[serial]
    fn construct_start_background_processes() -> Result<()> {
        // setup terrain.toml
        let test_dir = tempdir()?;
        let home_dir = tempdir()?;
        let test_dir_path: PathBuf = test_dir.path().into();
        let home_dir_path: PathBuf = home_dir.path().into();

        let paths = get_paths(home_dir_path, test_dir_path)?;

        let mut terrain_toml_path: PathBuf = test_dir.path().into();
        terrain_toml_path.push("terrain.toml");
        std::fs::copy("./tests/data/terrain.full.toml", &terrain_toml_path)?;

        // set selected biome
        let real_selected_biome = std::env::var("TERRAINIUM_SELECTED_BIOME").ok();
        std::env::set_var("TERRAINIUM_SELECTED_BIOME", "example_biome");

        let expected_commands: Vec<Command> = vec![Command {
            exe: "run".to_string(),
            args: Some(vec!["something".to_string()]),
        }];

        let mut expected_envs = BTreeMap::<String, String>::new();
        expected_envs.insert("EDITOR".to_string(), "nvim".to_string());
        expected_envs.insert("TEST".to_string(), "value".to_string());

        let start_background_process = mock_processes::start_context();
        start_background_process
            .expect()
            .withf(move |commands, envs| {
                let env_eq = *envs == expected_envs;
                let commands_eq = *commands == expected_commands;
                env_eq && commands_eq
            })
            .return_once(|_, _| Ok(()));

        super::build(Biome::get_constructors, &paths)?;

        // cleanup
        if let Some(selected_biome) = real_selected_biome {
            std::env::set_var("TERRAINIUM_SELECTED_BIOME", selected_biome)
        } else {
            std::env::remove_var("TERRAINIUM_SELECTED_BIOME")
        }
        Ok(())
    }

    #[test]
    #[serial]
    fn returns_err_if_constructor_background_process_spawn_has_error() -> Result<()> {
        // setup terrain.toml
        let test_dir = tempdir()?;
        let home_dir = tempdir()?;
        let test_dir_path: PathBuf = test_dir.path().into();
        let home_dir_path: PathBuf = home_dir.path().into();

        let paths = get_paths(home_dir_path, test_dir_path)?;

        let mut terrain_toml_path: PathBuf = test_dir.path().into();
        terrain_toml_path.push("terrain.toml");
        std::fs::copy("./tests/data/terrain.full.toml", &terrain_toml_path)?;

        // set selected biome
        let real_selected_biome = std::env::var("TERRAINIUM_SELECTED_BIOME").ok();
        std::env::set_var("TERRAINIUM_SELECTED_BIOME", "example_biome");

        let expected_commands: Vec<Command> = vec![Command {
            exe: "run".to_string(),
            args: Some(vec!["something".to_string()]),
        }];

        let mut expected_envs = BTreeMap::<String, String>::new();
        expected_envs.insert("EDITOR".to_string(), "nvim".to_string());
        expected_envs.insert("TEST".to_string(), "value".to_string());

        let start_background_process = mock_processes::start_context();
        start_background_process
            .expect()
            .withf(move |commands, envs| {
                let env_eq = *envs == expected_envs;
                let commands_eq = *commands == expected_commands;
                env_eq && commands_eq
            })
            .return_once(|_, _| Err(anyhow!("unable to run something")));

        let error = super::build(Biome::get_constructors, &paths)
            .unwrap_err()
            .to_string();

        assert_eq!("error while starting background processes", error);

        // cleanup
        if let Some(selected_biome) = real_selected_biome {
            std::env::set_var("TERRAINIUM_SELECTED_BIOME", selected_biome)
        } else {
            std::env::remove_var("TERRAINIUM_SELECTED_BIOME")
        }

        Ok(())
    }

    #[test]
    #[serial]
    fn deconstruct_starts_background_processes() -> Result<()> {
        // setup terrain.toml
        let test_dir = tempdir()?;
        let home_dir = tempdir()?;
        let test_dir_path: PathBuf = test_dir.path().into();
        let home_dir_path: PathBuf = home_dir.path().into();

        let paths = get_paths(home_dir_path, test_dir_path)?;

        let mut terrain_toml_path: PathBuf = test_dir.path().into();
        terrain_toml_path.push("terrain.toml");
        std::fs::copy("./tests/data/terrain.full.toml", &terrain_toml_path)?;

        // set selected biome
        let real_selected_biome = std::env::var("TERRAINIUM_SELECTED_BIOME").ok();
        std::env::set_var("TERRAINIUM_SELECTED_BIOME", "example_biome");

        let expected_commands: Vec<Command> = vec![Command {
            exe: "stop".to_string(),
            args: Some(vec!["something".to_string()]),
        }];

        let mut expected_envs = BTreeMap::<String, String>::new();
        expected_envs.insert("EDITOR".to_string(), "nvim".to_string());
        expected_envs.insert("TEST".to_string(), "value".to_string());

        let start_background_process = mock_processes::start_context();
        start_background_process
            .expect()
            .withf(move |commands, envs| {
                let env_eq = *envs == expected_envs;
                let commands_eq = *commands == expected_commands;
                env_eq && commands_eq
            })
            .return_once(|_, _| Ok(()));

        super::build(Biome::get_detructors, &paths)?;

        // cleanup
        if let Some(selected_biome) = real_selected_biome {
            std::env::set_var("TERRAINIUM_SELECTED_BIOME", selected_biome)
        } else {
            std::env::remove_var("TERRAINIUM_SELECTED_BIOME")
        }
        Ok(())
    }

    #[test]
    #[serial]
    fn returns_err_if_destructors_background_process_spawn_has_error() -> Result<()> {
        // setup terrain.toml
        let test_dir = tempdir()?;
        let home_dir = tempdir()?;
        let test_dir_path: PathBuf = test_dir.path().into();
        let home_dir_path: PathBuf = home_dir.path().into();

        let paths = get_paths(home_dir_path, test_dir_path)?;

        let mut terrain_toml_path: PathBuf = test_dir.path().into();
        terrain_toml_path.push("terrain.toml");
        std::fs::copy("./tests/data/terrain.full.toml", &terrain_toml_path)?;

        // set selected biome
        let real_selected_biome = std::env::var("TERRAINIUM_SELECTED_BIOME").ok();
        std::env::set_var("TERRAINIUM_SELECTED_BIOME", "example_biome");

        let expected_commands: Vec<Command> = vec![Command {
            exe: "stop".to_string(),
            args: Some(vec!["something".to_string()]),
        }];

        let mut expected_envs = BTreeMap::<String, String>::new();
        expected_envs.insert("EDITOR".to_string(), "nvim".to_string());
        expected_envs.insert("TEST".to_string(), "value".to_string());

        let start_background_process = mock_processes::start_context();
        start_background_process
            .expect()
            .withf(move |commands, envs| {
                let env_eq = *envs == expected_envs;
                let commands_eq = *commands == expected_commands;
                env_eq && commands_eq
            })
            .return_once(|_, _| Err(anyhow!("unable to stop something")));

        let error = super::build(Biome::get_detructors, &paths)
            .unwrap_err()
            .to_string();

        assert_eq!("error while starting background processes", error);

        // cleanup
        if let Some(selected_biome) = real_selected_biome {
            std::env::set_var("TERRAINIUM_SELECTED_BIOME", selected_biome)
        } else {
            std::env::remove_var("TERRAINIUM_SELECTED_BIOME")
        }

        Ok(())
    }
}
