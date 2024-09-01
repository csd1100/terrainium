use crate::helpers::utils::Paths;
use crate::types::biomes::Biome;
use anyhow::Result;

use super::build;

pub fn handle(paths: &Paths) -> Result<()> {
    build::build(Biome::get_constructors, paths)
}

#[cfg(test)]
mod test {
    use crate::helpers::utils::get_paths;
    use crate::{shell::background::mock_processes, types::commands::Command};
    use anyhow::{anyhow, Result};
    use serial_test::serial;
    use std::collections::BTreeMap;
    use std::path::PathBuf;
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

        let real_selected_biome = std::env::var("TERRAINIUM_SELECTED_BIOME").ok();
        std::env::set_var("TERRAINIUM_SELECTED_BIOME", "none");

        let expected_commands: Vec<Command> = vec![Command {
            exe: "run".to_string(),
            args: Some(vec!["something".to_string()]),
        }];

        let mut expected_envs = BTreeMap::<String, String>::new();
        expected_envs.insert("EDITOR".to_string(), "vim".to_string());
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

        super::handle(&paths)?;

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
    fn returns_err_if_background_process_spawn_has_error() -> Result<()> {
        // setup terrain.toml
        let test_dir = tempdir()?;
        let home_dir = tempdir()?;
        let test_dir_path: PathBuf = test_dir.path().into();
        let home_dir_path: PathBuf = home_dir.path().into();

        let paths = get_paths(home_dir_path, test_dir_path)?;

        let mut terrain_toml_path: PathBuf = test_dir.path().into();
        terrain_toml_path.push("terrain.toml");
        std::fs::copy("./tests/data/terrain.full.toml", &terrain_toml_path)?;

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

        let error = super::handle(&paths).unwrap_err().to_string();

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
