use std::collections::HashMap;

use anyhow::{Context, Result};
use mockall_double::double;
use uuid::Uuid;

use crate::{
    helpers::constants::{TERRAINIUM_ENABLED, TERRAINIUM_SESSION_ID},
    helpers::{
        constants::{
            TERRAINIUM_DEV, TERRAINIUM_EXECUTABLE_ENV, TERRAINIUM_SELECTED_BIOME,
            TERRAINIUM_TERRAIN_NAME, TERRAINIUM_TOML_PATH,
        },
        operations::merge_hashmaps,
    },
    types::args::BiomeArg,
};

#[double]
use crate::helpers::operations::fs;

#[double]
use crate::shell::zsh::ops;

pub fn handle(biome: Option<BiomeArg>) -> Result<()> {
    let toml_path = fs::get_current_dir_toml()?;
    let terrain = fs::get_parsed_terrain()?;
    let name: String = fs::get_terrain_name();

    let mut envs = HashMap::<String, String>::new();
    envs.insert(TERRAINIUM_ENABLED.to_string(), "true".to_string());
    envs.insert(
        TERRAINIUM_TOML_PATH.to_string(),
        toml_path.to_string_lossy().to_string(),
    );
    envs.insert(TERRAINIUM_TERRAIN_NAME.to_string(), name);
    envs.insert(
        TERRAINIUM_SESSION_ID.to_string(),
        Uuid::new_v4().to_string(),
    );
    envs.insert(
        TERRAINIUM_SELECTED_BIOME.to_string(),
        terrain.get_selected_biome_name(&biome)?,
    );

    let dev = std::env::var(TERRAINIUM_DEV);
    if dev.is_ok() && dev.unwrap() == *"true" {
        let mut pwd = std::env::current_dir().context("unable to get current_dir")?;
        pwd.push("target/debug/terrainium");

        envs.insert(
            TERRAINIUM_EXECUTABLE_ENV.to_string(),
            pwd.to_string_lossy().to_string(),
        );
    } else {
        envs.insert(
            TERRAINIUM_EXECUTABLE_ENV.to_string(),
            "terrainium".to_string(),
        );
    }

    let zsh_env = ops::get_zsh_envs(terrain.get_selected_biome_name(&biome)?)
        .context("unable to set zsh environment varibles")?;
    let mut merged = merge_hashmaps(&envs.clone(), &zsh_env);

    let selected = terrain
        .get(biome.clone())
        .context("unable to select biome")?;
    merged = merge_hashmaps(
        &merged,
        &selected.env.unwrap_or(HashMap::<String, String>::new()),
    );
    ops::spawn(vec!["-s"], Some(merged)).context("unable to start zsh")?;

    Ok(())
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, path::PathBuf};

    use anyhow::{Context, Result};
    use mockall::predicate::eq;
    use serial_test::serial;

    use crate::{
        helpers::{
            constants::{TERRAINIUM_DEV, TERRAINIUM_EXECUTABLE_ENV},
            operations::mock_fs,
        },
        shell::zsh::mock_ops,
        types::{args::BiomeArg, terrain::test_data},
    };

    #[test]
    #[serial]
    fn enter_enters_default() -> Result<()> {
        let mock_toml_path = mock_fs::get_current_dir_toml_context();
        mock_toml_path
            .expect()
            .return_once(|| Ok(PathBuf::from("./example_configs/terrain.full.toml")));

        let mock_name = mock_fs::get_terrain_name_context();
        mock_name.expect().return_const("test-terrain".to_string());

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
        expected.insert("EDITOR".to_string(), "nvim".to_string());
        expected.insert("TEST".to_string(), "value".to_string());
        expected.insert("TERRAINIUM_ENABLED".to_string(), "true".to_string());
        expected.insert(
            "TERRAINIUM_TERRAIN_NAME".to_string(),
            "test-terrain".to_string(),
        );
        expected.insert(
            "TERRAINIUM_TOML_PATH".to_string(),
            "./example_configs/terrain.full.toml".to_string(),
        );
        expected.insert(
            "TERRAINIUM_SELECTED_BIOME".to_string(),
            "example_biome".to_string(),
        );
        expected.insert("TERRAINIUM_SESSION_ID".to_string(), "1".to_string());
        let dev = std::env::var(TERRAINIUM_DEV);
        if dev.is_ok() && dev.unwrap() == *"true" {
            let mut pwd = std::env::current_dir().context("unable to get current_dir")?;
            pwd.push("target/debug/terrainium");

            expected.insert(
                TERRAINIUM_EXECUTABLE_ENV.to_string(),
                pwd.to_string_lossy().to_string(),
            );
        } else {
            expected.insert(
                TERRAINIUM_EXECUTABLE_ENV.to_string(),
                "terrainium".to_string(),
            );
        }

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
        let mock_toml_path = mock_fs::get_current_dir_toml_context();
        mock_toml_path
            .expect()
            .return_once(|| Ok(PathBuf::from("./example_configs/terrain.full.toml")));

        let mock_name = mock_fs::get_terrain_name_context();
        mock_name.expect().return_const("test-terrain".to_string());

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
        expected.insert("EDITOR".to_string(), "nano".to_string());
        expected.insert("TEST".to_string(), "value".to_string());
        expected.insert("TERRAINIUM_ENABLED".to_string(), "true".to_string());
        expected.insert(
            "TERRAINIUM_TERRAIN_NAME".to_string(),
            "test-terrain".to_string(),
        );
        expected.insert(
            "TERRAINIUM_TOML_PATH".to_string(),
            "./example_configs/terrain.full.toml".to_string(),
        );
        expected.insert(
            "TERRAINIUM_SELECTED_BIOME".to_string(),
            "example_biome2".to_string(),
        );
        expected.insert("TERRAINIUM_SESSION_ID".to_string(), "1".to_string());
        let dev = std::env::var(TERRAINIUM_DEV);
        if dev.is_ok() && dev.unwrap() == *"true" {
            let mut pwd = std::env::current_dir().context("unable to get current_dir")?;
            pwd.push("target/debug/terrainium");

            expected.insert(
                TERRAINIUM_EXECUTABLE_ENV.to_string(),
                pwd.to_string_lossy().to_string(),
            );
        } else {
            expected.insert(
                TERRAINIUM_EXECUTABLE_ENV.to_string(),
                "terrainium".to_string(),
            );
        }

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
        let mock_toml_path = mock_fs::get_current_dir_toml_context();
        mock_toml_path
            .expect()
            .return_once(|| Ok(PathBuf::from("./example_configs/terrain.full.toml")));

        let mock_name = mock_fs::get_terrain_name_context();
        mock_name.expect().return_const("test-terrain".to_string());

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
        expected.insert("VAR1".to_string(), "val1".to_string());
        expected.insert("VAR2".to_string(), "val2".to_string());
        expected.insert("VAR3".to_string(), "val3".to_string());
        expected.insert("TERRAINIUM_ENABLED".to_string(), "true".to_string());
        expected.insert(
            "TERRAINIUM_TERRAIN_NAME".to_string(),
            "test-terrain".to_string(),
        );
        expected.insert(
            "TERRAINIUM_TOML_PATH".to_string(),
            "./example_configs/terrain.full.toml".to_string(),
        );
        expected.insert("TERRAINIUM_SELECTED_BIOME".to_string(), "none".to_string());
        expected.insert("TERRAINIUM_SESSION_ID".to_string(), "1".to_string());
        let dev = std::env::var(TERRAINIUM_DEV);
        if dev.is_ok() && dev.unwrap() == *"true" {
            let mut pwd = std::env::current_dir().context("unable to get current_dir")?;
            pwd.push("target/debug/terrainium");

            expected.insert(
                TERRAINIUM_EXECUTABLE_ENV.to_string(),
                pwd.to_string_lossy().to_string(),
            );
        } else {
            expected.insert(
                TERRAINIUM_EXECUTABLE_ENV.to_string(),
                "terrainium".to_string(),
            );
        }

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
