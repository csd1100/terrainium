use anyhow::Result;

use crate::types::biomes::Biome;

use super::build;

pub fn handle() -> Result<()> {
    build::build(Biome::get_constructors)
}

// #[cfg(test)]
// mod test {
//     use std::{collections::HashMap, path::PathBuf};
//
//     use anyhow::{anyhow, Result};
//     use mockall::predicate::eq;
//     use serial_test::serial;
//
//     use crate::{
//         helpers::operations::mock_fs,
//         shell::background::mock_processes,
//         types::{args::BiomeArg, commands::Command},
//     };
//
//     #[test]
//     #[serial]
//     fn construct_start_background_processes() -> Result<()> {
//         let real_selected_biome = std::env::var("TERRAINIUM_SELECTED_BIOME").ok();
//         std::env::set_var("TERRAINIUM_SELECTED_BIOME", "none");
//
//         let mock_terrain_toml = mock_fs::get_terrain_toml_from_biome_context();
//         mock_terrain_toml
//             .expect()
//             .with(eq(Some(BiomeArg::None)))
//             .return_once(|_| Ok(PathBuf::from("./example_configs/terrain.full.toml")))
//             .times(1);
//
//         let expected_commands: Vec<Command> = vec![Command {
//             exe: "run".to_string(),
//             args: Some(vec!["something".to_string()]),
//         }];
//
//         let mut expected_envs = HashMap::<String, String>::new();
//         expected_envs.insert("EDITOR".to_string(), "vim".to_string());
//         expected_envs.insert("TEST".to_string(), "value".to_string());
//
//         let start_background_process = mock_processes::start_context();
//         start_background_process
//             .expect()
//             .withf(move |commands, envs| {
//                 let env_eq = *envs == expected_envs;
//                 let commands_eq = *commands == expected_commands;
//                 env_eq && commands_eq
//             })
//             .return_once(|_, _| Ok(()));
//
//         super::handle()?;
//
//         // cleanup
//         if let Some(selected_biome) = real_selected_biome {
//             std::env::set_var("TERRAINIUM_SELECTED_BIOME", selected_biome)
//         } else {
//             std::env::remove_var("TERRAINIUM_SELECTED_BIOME")
//         }
//         Ok(())
//     }
//
//     #[test]
//     #[serial]
//     fn returns_err_if_background_process_spawn_has_error() -> Result<()> {
//         let real_selected_biome = std::env::var("TERRAINIUM_SELECTED_BIOME").ok();
//         std::env::set_var("TERRAINIUM_SELECTED_BIOME", "example_biome");
//
//         let mock_terrain_toml = mock_fs::get_terrain_toml_from_biome_context();
//         mock_terrain_toml
//             .expect()
//             .with(eq(Some(BiomeArg::Value("example_biome".to_string()))))
//             .return_once(|_| Ok(PathBuf::from("./example_configs/terrain.full.toml")))
//             .times(1);
//
//         let expected_commands: Vec<Command> = vec![Command {
//             exe: "run".to_string(),
//             args: Some(vec!["something".to_string()]),
//         }];
//
//         let mut expected_envs = HashMap::<String, String>::new();
//         expected_envs.insert("EDITOR".to_string(), "nvim".to_string());
//         expected_envs.insert("TEST".to_string(), "value".to_string());
//
//         let start_background_process = mock_processes::start_context();
//         start_background_process
//             .expect()
//             .withf(move |commands, envs| {
//                 let env_eq = *envs == expected_envs;
//                 let commands_eq = *commands == expected_commands;
//                 env_eq && commands_eq
//             })
//             .return_once(|_, _| Err(anyhow!("unable to run something")));
//
//         let error = super::handle().unwrap_err().to_string();
//
//         assert_eq!("error while starting background processes", error);
//
//         // cleanup
//         if let Some(selected_biome) = real_selected_biome {
//             std::env::set_var("TERRAINIUM_SELECTED_BIOME", selected_biome)
//         } else {
//             std::env::remove_var("TERRAINIUM_SELECTED_BIOME")
//         }
//
//         Ok(())
//     }
// }
