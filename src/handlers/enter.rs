use std::collections::HashMap;

use anyhow::{Context, Result};
use mockall_double::double;
use uuid::Uuid;

use crate::{
    helpers::constants::{TERRAINIUM_ENABLED, TERRAINIUM_SESSION_ID},
    helpers::helpers::merge_hashmaps,
    types::args::BiomeArg,
};

#[double]
use super::construct::run;

#[double]
use crate::helpers::helpers::fs;

#[double]
use crate::shell::zsh::ops;

pub fn handle(biome: Option<BiomeArg>) -> Result<()> {
    let terrain = fs::get_parsed_terrain()?;
    let selected = terrain
        .get(biome.clone())
        .context("unable to select biome")?;

    let mut envs = HashMap::<String, String>::new();
    envs.insert(TERRAINIUM_ENABLED.to_string(), "1".to_string());
    envs.insert(
        TERRAINIUM_SESSION_ID.to_string(),
        Uuid::new_v4().to_string(),
    );
    let zsh_env = ops::get_zsh_envs(terrain.get_selected_biome_name(&biome)?)
        .context("unable to set zsh environment varibles")?;
    let mut merged = merge_hashmaps(&envs.clone(), &zsh_env);

    run::constructors(biome, Some(&merged)).context("unable to construct biome")?;

    if let Some(biome_env) = selected.env {
        merged = merge_hashmaps(&merged, &biome_env);
    }
    ops::spawn(vec!["-s"], Some(merged)).context("unable to start zsh")?;

    Ok(())
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use anyhow::Result;
    use mockall::predicate::eq;
    use serial_test::serial;

    use crate::{
        handlers::construct::mock_run,
        helpers::helpers::mock_fs,
        shell::zsh::mock_ops,
        types::{args::BiomeArg, terrain::test_data},
    };

    #[test]
    #[serial]
    fn enter_enters_default() -> Result<()> {
        let mock_terrain = mock_fs::get_parsed_terrain_context();
        mock_terrain
            .expect()
            .return_once(|| Ok(test_data::terrain_full()))
            .times(1);

        let mock_zsh_env = mock_ops::get_zsh_envs_context();
        mock_zsh_env
            .expect()
            .with(eq("example_biome".to_string()))
            .return_once(|_| Ok(HashMap::<String, String>::new()))
            .times(1);

        let mut expected = HashMap::<String, String>::new();
        expected.insert("TERRAINIUM_ENABLED".to_string(), "1".to_string());
        expected.insert("TERRAINIUM_SESSION_ID".to_string(), "1".to_string());

        // do not validate TERRAINIUM_SESSION_ID as it is uuid
        let mock_constructors = mock_run::constructors_context();
        mock_constructors
            .expect()
            .withf(move |biome, envs| {
                let biome_eq = biome.is_none();
                let env_len_eq = expected.len() == envs.unwrap().len();
                expected.iter().for_each(|(exp_k, exp_v)| {
                    if exp_k != "TERRAINIUM_SESSION_ID" {
                        assert_eq!(exp_v, envs.unwrap().get(exp_k).expect("to be present"));
                    }
                });
                biome_eq && env_len_eq
            })
            .return_once(|_, _| Ok(()))
            .times(1);

        let mut expected = HashMap::<String, String>::new();
        expected.insert("EDITOR".to_string(), "nvim".to_string());
        expected.insert("TEST".to_string(), "value".to_string());
        expected.insert("TERRAINIUM_ENABLED".to_string(), "1".to_string());
        expected.insert("TERRAINIUM_SESSION_ID".to_string(), "1".to_string());

        // do not validate TERRAINIUM_SESSION_ID as it is uuid
        let mock_spawn = mock_ops::spawn_context();
        mock_spawn
            .expect()
            .withf(move |args, envs| {
                let args_eq = *args == vec!["-s"];
                let env_len_eq = expected.len() == envs.as_ref().unwrap().len();
                expected.iter().for_each(|(exp_k, exp_v)| {
                    if exp_k != "TERRAINIUM_SESSION_ID" {
                        assert_eq!(
                            exp_v,
                            envs.as_ref().unwrap().get(exp_k).expect("to be present")
                        );
                    }
                });
                args_eq && env_len_eq
            })
            .return_once(|_, _| Ok(()))
            .times(1);

        super::handle(None)?;
        Ok(())
    }

    #[test]
    #[serial]
    fn enter_enters_selected() -> Result<()> {
        let mock_terrain = mock_fs::get_parsed_terrain_context();
        mock_terrain
            .expect()
            .return_once(|| Ok(test_data::terrain_full()))
            .times(1);

        let mock_zsh_env = mock_ops::get_zsh_envs_context();
        mock_zsh_env
            .expect()
            .with(eq("example_biome2".to_string()))
            .return_once(|_| Ok(HashMap::<String, String>::new()))
            .times(1);

        let mut expected = HashMap::<String, String>::new();
        expected.insert("TERRAINIUM_ENABLED".to_string(), "1".to_string());
        expected.insert("TERRAINIUM_SESSION_ID".to_string(), "1".to_string());

        // do not validate TERRAINIUM_SESSION_ID as it is uuid
        let mock_constructors = mock_run::constructors_context();
        mock_constructors
            .expect()
            .withf(move |biome, envs| {
                let biome_eq = *biome == Some(BiomeArg::Value("example_biome2".to_string()));
                let env_len_eq = envs.unwrap().len() == expected.len();
                expected.iter().for_each(|(exp_k, exp_v)| {
                    if exp_k != "TERRAINIUM_SESSION_ID" {
                        assert_eq!(exp_v, envs.unwrap().get(exp_k).expect("to be present"));
                    }
                });
                biome_eq && env_len_eq
            })
            .return_once(|_, _| Ok(()))
            .times(1);

        let mut expected = HashMap::<String, String>::new();
        expected.insert("EDITOR".to_string(), "nano".to_string());
        expected.insert("TEST".to_string(), "value".to_string());
        expected.insert("TERRAINIUM_ENABLED".to_string(), "1".to_string());
        expected.insert("TERRAINIUM_SESSION_ID".to_string(), "1".to_string());

        // do not validate TERRAINIUM_SESSION_ID as it is uuid
        let mock_spawn = mock_ops::spawn_context();
        mock_spawn
            .expect()
            .withf(move |args, envs| {
                let args_eq = *args == vec!["-s"];
                let env_len_eq = envs.as_ref().unwrap().len() == expected.len();
                expected.iter().for_each(|(exp_k, exp_v)| {
                    if exp_k != "TERRAINIUM_SESSION_ID" {
                        assert_eq!(
                            exp_v,
                            envs.as_ref().unwrap().get(exp_k).expect("to be present")
                        );
                    }
                });
                args_eq && env_len_eq
            })
            .return_once(|_, _| Ok(()))
            .times(1);

        super::handle(Some(BiomeArg::Value("example_biome2".to_string())))?;
        Ok(())
    }

    #[test]
    #[serial]
    fn enter_enters_main() -> Result<()> {
        let mock_terrain = mock_fs::get_parsed_terrain_context();
        mock_terrain
            .expect()
            .return_once(|| Ok(test_data::terrain_without_biomes()))
            .times(1);

        let mock_zsh_env = mock_ops::get_zsh_envs_context();
        mock_zsh_env
            .expect()
            .with(eq("none".to_string()))
            .return_once(|_| Ok(HashMap::<String, String>::new()))
            .times(1);

        let mut expected = HashMap::<String, String>::new();
        expected.insert("TERRAINIUM_ENABLED".to_string(), "1".to_string());
        expected.insert("TERRAINIUM_SESSION_ID".to_string(), "1".to_string());

        // do not validate TERRAINIUM_SESSION_ID as it is uuid
        let mock_constructors = mock_run::constructors_context();
        mock_constructors
            .expect()
            .withf(move |biome, envs| {
                let biome_eq = biome.is_none();

                let env_len_eq = envs.unwrap().len() == expected.len();
                expected.iter().for_each(|(exp_k, exp_v)| {
                    if exp_k != "TERRAINIUM_SESSION_ID" {
                        assert_eq!(exp_v, envs.unwrap().get(exp_k).expect("to be present"));
                    }
                });
                biome_eq && env_len_eq
            })
            .return_once(|_, _| Ok(()))
            .times(1);

        let mut expected = HashMap::<String, String>::new();
        expected.insert("VAR1".to_string(), "val1".to_string());
        expected.insert("VAR2".to_string(), "val2".to_string());
        expected.insert("VAR3".to_string(), "val3".to_string());
        expected.insert("TERRAINIUM_ENABLED".to_string(), "1".to_string());
        expected.insert("TERRAINIUM_SESSION_ID".to_string(), "1".to_string());

        // do not validate TERRAINIUM_SESSION_ID as it is uuid
        let mock_spawn = mock_ops::spawn_context();
        mock_spawn
            .expect()
            .withf(move |args, envs| {
                let args_eq = *args == vec!["-s"];
                let env_len_eq = envs.as_ref().unwrap().len() == expected.len();
                expected.iter().for_each(|(exp_k, exp_v)| {
                    if exp_k != "TERRAINIUM_SESSION_ID" {
                        assert_eq!(
                            exp_v,
                            envs.as_ref().unwrap().get(exp_k).expect("to be present")
                        );
                    }
                });
                args_eq && env_len_eq
            })
            .return_once(|_, _| Ok(()))
            .times(1);

        super::handle(None)?;
        Ok(())
    }
}
