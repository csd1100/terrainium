use anyhow::Result;

#[cfg(test)]
use mockall::automock;

use crate::types::args::BiomeArg;

pub fn handle(biome: Option<BiomeArg>) -> Result<()> {
    run::constructors(biome, None)
}

#[cfg_attr(test, automock)]
pub mod run {
    use anyhow::{Context, Result};
    use mockall_double::double;
    use std::collections::HashMap;

    use crate::types::args::BiomeArg;

    #[double]
    use crate::shell::background::processes;

    #[double]
    use crate::helpers::helpers::fs;
    use crate::helpers::helpers::merge_hashmaps;
    use crate::types::biomes::Biome;

    #[allow(clippy::needless_lifetimes)]
    pub fn constructors<'a>(
        biome: Option<BiomeArg>,
        envs: Option<&'a HashMap<String, String>>,
    ) -> Result<()> {
        let terrain: Biome = fs::get_parsed_terrain()?
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
    use std::collections::HashMap;

    use anyhow::{anyhow, Result};
    use serial_test::serial;

    use crate::{
        helpers::helpers::mock_fs,
        shell::background::mock_processes,
        types::{commands::Command, terrain::test_data},
    };

    #[test]
    #[serial]
    fn construct_start_background_processes() -> Result<()> {
        let mock_terrain = mock_fs::get_parsed_terrain_context();
        mock_terrain
            .expect()
            .return_once(|| Ok(test_data::terrain_full()))
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

        super::handle(None)?;

        Ok(())
    }

    #[test]
    #[serial]
    fn returns_err_if_background_process_spawn_has_error() -> Result<()> {
        let mock_terrain = mock_fs::get_parsed_terrain_context();
        mock_terrain
            .expect()
            .return_once(|| Ok(test_data::terrain_full()))
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

        let error = super::handle(None).unwrap_err().to_string();

        assert_eq!("error while starting background processes", error);

        Ok(())
    }
}
