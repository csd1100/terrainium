use anyhow::{anyhow, Result};

#[cfg(test)]
use mockall::automock;

use crate::{helpers::constants::TERRAINIUM_SELECTED_BIOME, types::args::BiomeArg};

pub fn handle() -> Result<()> {
    let biome = if let std::result::Result::Ok(current) = std::env::var(TERRAINIUM_SELECTED_BIOME) {
        BiomeArg::Current(current)
    } else {
        return Err(anyhow!("no active biome found"));
    };
    run::constructors(Some(biome), None)
}

#[cfg_attr(test, automock)]
pub mod run {
    use anyhow::{Context, Result};
    use mockall_double::double;
    use std::collections::HashMap;

    use crate::helpers::operations::merge_hashmaps;
    use crate::types::args::BiomeArg;
    use crate::types::terrain;

    #[double]
    use crate::shell::background::processes;

    #[double]
    use crate::helpers::operations::fs;

    #[allow(clippy::needless_lifetimes)]
    pub fn constructors<'a>(
        biome: Option<BiomeArg>,
        envs: Option<&'a HashMap<String, String>>,
    ) -> Result<()> {
        let terrain_toml = fs::get_terrain_toml_from_biome(&biome)?;
        let terrain = terrain::parse_terrain(&terrain_toml)?
            .get(biome)
            .context("unable to select a biome to call constructors")?;

        let envs = merge_hashmaps(
            &terrain.env.unwrap_or(HashMap::<String, String>::new()),
            envs.unwrap_or(&HashMap::<String, String>::new()),
        );

        terrain
            .constructors
            .and_then(|constructors| {
                constructors
                    .background
                    .map(|commands| Some(processes::start(commands, envs)))
            })
            .flatten()
            .transpose()
            .context("error while starting background processes")?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, path::PathBuf};

    use anyhow::{anyhow, Result};
    use mockall::predicate::eq;
    use serial_test::serial;

    use crate::{
        helpers::operations::mock_fs,
        shell::background::mock_processes,
        types::{args::BiomeArg, commands::Command},
    };

    #[test]
    #[serial]
    fn construct_start_background_processes() -> Result<()> {
        let real_selected_biome = std::env::var("TERRAINIUM_SELECTED_BIOME").ok();
        std::env::set_var("TERRAINIUM_SELECTED_BIOME", "example_biome");

        let mock_terrain_toml = mock_fs::get_terrain_toml_from_biome_context();
        mock_terrain_toml
            .expect()
            .with(eq(Some(BiomeArg::Current("example_biome".to_string()))))
            .return_once(|_| Ok(PathBuf::from("./example_configs/terrain.full.toml")))
            .times(1);

        let expected_commands: Vec<Command> = vec![Command {
            exe: "run".to_string(),
            args: Some(vec!["something".to_string()]),
        }];

        let mut expected_envs = HashMap::<String, String>::new();
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

        super::handle()?;

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
        let real_selected_biome = std::env::var("TERRAINIUM_SELECTED_BIOME").ok();
        std::env::set_var("TERRAINIUM_SELECTED_BIOME", "example_biome");

        let mock_terrain_toml = mock_fs::get_terrain_toml_from_biome_context();
        mock_terrain_toml
            .expect()
            .with(eq(Some(BiomeArg::Current("example_biome".to_string()))))
            .return_once(|_| Ok(PathBuf::from("./example_configs/terrain.full.toml")))
            .times(1);

        let expected_commands: Vec<Command> = vec![Command {
            exe: "run".to_string(),
            args: Some(vec!["something".to_string()]),
        }];

        let mut expected_envs = HashMap::<String, String>::new();
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

        let error = super::handle().unwrap_err().to_string();

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
