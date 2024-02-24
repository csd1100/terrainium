use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use mockall_double::double;
use prost::Message;

use crate::{
    helpers::{
        constants::{
            TERRAINIUM_DEV, TERRAINIUM_ENABLED, TERRAINIUM_EXECUTABLE_ENV, TERRAINIUM_EXECUTOR_ENV,
            TERRAINIUM_SELECTED_BIOME, TERRAINIUM_SESSION_ID, TERRAINIUM_TERRAIN_NAME,
            TERRAINIUM_TOML_PATH,
        },
        operations::{get_current_dir_toml, get_parsed_terrain, get_terrain_name, merge_hashmaps},
    },
    proto::{self, ActivateRequest},
    types::args::BiomeArg,
};

#[double]
use crate::helpers::utils::fs;

#[double]
use crate::helpers::utils::misc;

#[double]
use crate::types::socket::Unix;

#[double]
use crate::shell::zsh::ops;

fn add_executables(map: &mut HashMap<String, String>) -> Result<()> {
    let dev = std::env::var(TERRAINIUM_DEV);
    if dev.is_ok() && dev.unwrap() == *"true" {
        let pwd = fs::get_cwd().context("unable to get current_dir")?;
        let mut terrainium = pwd.clone();
        terrainium.push("target/debug/terrainium");
        let mut terrainium_executor = pwd.clone();
        terrainium_executor.push("target/debug/terrainium_executor");

        map.insert(
            TERRAINIUM_EXECUTABLE_ENV.to_string(),
            terrainium.to_string_lossy().to_string(),
        );
        map.insert(
            TERRAINIUM_EXECUTOR_ENV.to_string(),
            terrainium_executor.to_string_lossy().to_string(),
        );
        Ok(())
    } else {
        map.insert(
            TERRAINIUM_EXECUTABLE_ENV.to_string(),
            "terrainium".to_string(),
        );
        map.insert(
            TERRAINIUM_EXECUTOR_ENV.to_string(),
            "terrainium_executor".to_string(),
        );
        Ok(())
    }
}

pub fn handle(biome: Option<BiomeArg>) -> Result<()> {
    let enabled = std::env::var(TERRAINIUM_ENABLED);
    if enabled.is_ok() && enabled.unwrap() == *"true" {
        return Err(anyhow!("other terrain is already active"));
    }

    let terrain = get_parsed_terrain()?;

    let terrain_name: String = get_terrain_name();
    let toml_path = get_current_dir_toml()?.to_string_lossy().to_string();
    let session_id = misc::get_uuid();
    let biome_name = terrain.get_selected_biome_name(&biome)?;

    let mut envs = HashMap::<String, String>::new();
    envs.insert(TERRAINIUM_ENABLED.to_string(), "true".to_string());
    envs.insert(TERRAINIUM_TOML_PATH.to_string(), toml_path.clone());
    envs.insert(TERRAINIUM_TERRAIN_NAME.to_string(), terrain_name.clone());
    envs.insert(TERRAINIUM_SESSION_ID.to_string(), session_id.clone());
    envs.insert(TERRAINIUM_SELECTED_BIOME.to_string(), biome_name.clone());
    add_executables(&mut envs)?;

    let mut socket = Unix::new()?;
    socket.write(proto::Request {
        session: Some(proto::request::Session::SessionId(session_id)),
        args: Some(proto::request::Args::Activate(ActivateRequest {
            terrain_name,
            biome_name,
            toml_path,
        })),
    })?;

    let response = socket
        .read()
        .context("error while reading response from daemon")?;
    let response = proto::Response::decode(response).context("error while decoding response")?;
    let result = response.result.ok_or(anyhow!("no result in response"))?;

    match result {
        proto::response::Result::Success(_) => {}
        proto::response::Result::Error(err) => {
            return Err(anyhow!(format!(
                "error in operation: {}",
                err.error_message
            )));
        }
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

// #[cfg(test)]
// mod test {
//     use std::{collections::HashMap, path::PathBuf};
//
//     use anyhow::{Context, Result};
//     use mockall::predicate::eq;
//     use prost::{bytes::BytesMut, Message};
//     use serial_test::serial;
//
//     use crate::{
//         helpers::{
//             constants::{TERRAINIUM_DEV, TERRAINIUM_EXECUTABLE_ENV, TERRAINIUM_EXECUTOR_ENV},
//             operations::{mock_fs, mock_misc},
//         },
//         proto,
//         shell::zsh::mock_ops,
//         types::{args::BiomeArg, socket::MockUnix, terrain::test_data},
//     };
//
//     #[test]
//     #[serial]
//     fn enter_enters_default() -> Result<()> {
//         let enabled = std::env::var("TERRAINIUM_ENABLED").ok();
//         std::env::remove_var("TERRAINIUM_ENABLED");
//
//         let mock_toml_path = mock_fs::get_current_dir_toml_context();
//         mock_toml_path
//             .expect()
//             .return_once(|| Ok(PathBuf::from("./example_configs/terrain.full.toml")));
//
//         let mock_name = mock_fs::get_terrain_name_context();
//         mock_name.expect().return_const("test-terrain".to_string());
//
//         let session_id = String::from("session_id");
//         let mock_session_id = mock_misc::get_uuid_context();
//         mock_session_id.expect().return_const(session_id.clone());
//
//         let mock_terrain = mock_fs::get_parsed_terrain_context();
//         mock_terrain
//             .expect()
//             .return_once(|| Ok(test_data::terrain_full()))
//             .times(1);
//
//         let mocket_new = MockUnix::new_context();
//         mocket_new.expect().returning(move || {
//             let requst = proto::Request {
//                 session: Some(proto::request::Session::SessionId("session_id".to_string())),
//                 args: Some(proto::request::Args::Activate(proto::ActivateRequest {
//                     terrain_name: "test-terrain".to_string(),
//                     biome_name: "example_biome".to_string(),
//                     toml_path: "./example_configs/terrain.full.toml".to_string(),
//                 })),
//             };
//             let mut mocket = MockUnix::default();
//             mocket
//                 .expect_write::<proto::Request>()
//                 .with(eq(requst))
//                 .return_once(|_| Ok(()));
//             mocket.expect_read().return_once(|| {
//                 let mut buf = BytesMut::new();
//                 proto::Response {
//                     result: Some(proto::response::Result::Success(proto::response::Success {
//                         body: Some(proto::response::success::Body::Activate(
//                             proto::ActivateResponse {},
//                         )),
//                     })),
//                 }
//                 .encode(&mut buf)?;
//                 Ok(buf.into())
//             });
//             Ok(mocket)
//         });
//
//         let mock_zsh_env = mock_ops::get_zsh_envs_context();
//         mock_zsh_env
//             .expect()
//             .with(eq("example_biome".to_string()))
//             .return_once(|_| Ok(HashMap::<String, String>::new()))
//             .times(1);
//
//         let mut expected = HashMap::<String, String>::new();
//         expected.insert("EDITOR".to_string(), "nvim".to_string());
//         expected.insert("TEST".to_string(), "value".to_string());
//         expected.insert("TERRAINIUM_ENABLED".to_string(), "true".to_string());
//         expected.insert(
//             "TERRAINIUM_TERRAIN_NAME".to_string(),
//             "test-terrain".to_string(),
//         );
//         expected.insert(
//             "TERRAINIUM_TOML_PATH".to_string(),
//             "./example_configs/terrain.full.toml".to_string(),
//         );
//         expected.insert(
//             "TERRAINIUM_SELECTED_BIOME".to_string(),
//             "example_biome".to_string(),
//         );
//         expected.insert("TERRAINIUM_SESSION_ID".to_string(), session_id);
//
//         let dev = std::env::var(TERRAINIUM_DEV);
//         if dev.is_ok() && dev.unwrap() == *"true" {
//             let pwd = std::env::current_dir().context("unable to get current_dir")?;
//             let mut terrainium = pwd.clone();
//             terrainium.push("target/debug/terrainium");
//             let mut terrainium_executor = pwd.clone();
//             terrainium_executor.push("target/debug/terrainium_executor");
//
//             expected.insert(
//                 TERRAINIUM_EXECUTABLE_ENV.to_string(),
//                 terrainium.to_string_lossy().to_string(),
//             );
//             expected.insert(
//                 TERRAINIUM_EXECUTOR_ENV.to_string(),
//                 terrainium_executor.to_string_lossy().to_string(),
//             );
//         } else {
//             expected.insert(
//                 TERRAINIUM_EXECUTABLE_ENV.to_string(),
//                 "terrainium".to_string(),
//             );
//             expected.insert(
//                 TERRAINIUM_EXECUTOR_ENV.to_string(),
//                 "terrainium_executor".to_string(),
//             );
//         }
//
//         let mock_spawn = mock_ops::spawn_context();
//         mock_spawn
//             .expect()
//             .withf(move |args, envs| {
//                 let args_eq = *args == vec!["-s"];
//                 let env_eq = *envs.as_ref().unwrap() == expected;
//                 args_eq && env_eq
//             })
//             .return_once(|_, _| Ok(()))
//             .times(1);
//
//         super::handle(None)?;
//
//         // cleanup
//         if let Some(enabled) = enabled {
//             std::env::set_var("TERRAINIUM_ENABLED", enabled)
//         } else {
//             std::env::remove_var("TERRAINIUM_ENABLED")
//         }
//         Ok(())
//     }
//
//     #[test]
//     #[serial]
//     fn enter_enters_selected() -> Result<()> {
//         let enabled = std::env::var("TERRAINIUM_ENABLED").ok();
//         std::env::remove_var("TERRAINIUM_ENABLED");
//
//         let mock_toml_path = mock_fs::get_current_dir_toml_context();
//         mock_toml_path
//             .expect()
//             .return_once(|| Ok(PathBuf::from("./example_configs/terrain.full.toml")));
//
//         let mock_name = mock_fs::get_terrain_name_context();
//         mock_name.expect().return_const("test-terrain".to_string());
//
//         let session_id = String::from("session_id");
//         let mock_session_id = mock_misc::get_uuid_context();
//         mock_session_id.expect().return_const(session_id.clone());
//
//         let mock_terrain = mock_fs::get_parsed_terrain_context();
//         mock_terrain
//             .expect()
//             .return_once(|| Ok(test_data::terrain_full()))
//             .times(1);
//
//         let mocket_new = MockUnix::new_context();
//         mocket_new.expect().returning(|| {
//             let request = proto::Request {
//                 session: Some(proto::request::Session::SessionId("session_id".to_string())),
//                 args: Some(proto::request::Args::Activate(proto::ActivateRequest {
//                     terrain_name: "test-terrain".to_string(),
//                     biome_name: "example_biome2".to_string(),
//                     toml_path: "./example_configs/terrain.full.toml".to_string(),
//                 })),
//             };
//             let mut mocket = MockUnix::default();
//             mocket
//                 .expect_write::<proto::Request>()
//                 .with(eq(request))
//                 .return_once(|_| Ok(()));
//             mocket.expect_read().return_once(|| {
//                 let mut buf = BytesMut::new();
//                 proto::Response {
//                     result: Some(proto::response::Result::Success(proto::response::Success {
//                         body: Some(proto::response::success::Body::Activate(
//                             proto::ActivateResponse {},
//                         )),
//                     })),
//                 }
//                 .encode(&mut buf)?;
//                 Ok(buf.into())
//             });
//             Ok(mocket)
//         });
//
//         let mock_zsh_env = mock_ops::get_zsh_envs_context();
//         mock_zsh_env
//             .expect()
//             .with(eq("example_biome2".to_string()))
//             .return_once(|_| Ok(HashMap::<String, String>::new()))
//             .times(1);
//
//         let mut expected = HashMap::<String, String>::new();
//         expected.insert("EDITOR".to_string(), "nano".to_string());
//         expected.insert("TEST".to_string(), "value".to_string());
//         expected.insert("TERRAINIUM_ENABLED".to_string(), "true".to_string());
//         expected.insert(
//             "TERRAINIUM_TERRAIN_NAME".to_string(),
//             "test-terrain".to_string(),
//         );
//         expected.insert(
//             "TERRAINIUM_TOML_PATH".to_string(),
//             "./example_configs/terrain.full.toml".to_string(),
//         );
//         expected.insert(
//             "TERRAINIUM_SELECTED_BIOME".to_string(),
//             "example_biome2".to_string(),
//         );
//         expected.insert(
//             "TERRAINIUM_SESSION_ID".to_string(),
//             "session_id".to_string(),
//         );
//
//         let dev = std::env::var(TERRAINIUM_DEV);
//         if dev.is_ok() && dev.unwrap() == *"true" {
//             let pwd = std::env::current_dir().context("unable to get current_dir")?;
//             let mut terrainium = pwd.clone();
//             terrainium.push("target/debug/terrainium");
//             let mut terrainium_executor = pwd.clone();
//             terrainium_executor.push("target/debug/terrainium_executor");
//
//             expected.insert(
//                 TERRAINIUM_EXECUTABLE_ENV.to_string(),
//                 terrainium.to_string_lossy().to_string(),
//             );
//             expected.insert(
//                 TERRAINIUM_EXECUTOR_ENV.to_string(),
//                 terrainium_executor.to_string_lossy().to_string(),
//             );
//         } else {
//             expected.insert(
//                 TERRAINIUM_EXECUTABLE_ENV.to_string(),
//                 "terrainium".to_string(),
//             );
//             expected.insert(
//                 TERRAINIUM_EXECUTOR_ENV.to_string(),
//                 "terrainium_executor".to_string(),
//             );
//         }
//
//         let mock_spawn = mock_ops::spawn_context();
//         mock_spawn
//             .expect()
//             .withf(move |args, envs| {
//                 let args_eq = *args == vec!["-s"];
//                 let env_eq = *envs.as_ref().unwrap() == expected;
//                 args_eq && env_eq
//             })
//             .return_once(|_, _| Ok(()))
//             .times(1);
//
//         super::handle(Some(BiomeArg::Value("example_biome2".to_string())))?;
//
//         // cleanup
//         if let Some(enabled) = enabled {
//             std::env::set_var("TERRAINIUM_ENABLED", enabled)
//         } else {
//             std::env::remove_var("TERRAINIUM_ENABLED")
//         }
//         Ok(())
//     }
//
//     #[test]
//     #[serial]
//     fn enter_enters_main() -> Result<()> {
//         let enabled = std::env::var("TERRAINIUM_ENABLED").ok();
//         std::env::remove_var("TERRAINIUM_ENABLED");
//
//         let mock_toml_path = mock_fs::get_current_dir_toml_context();
//         mock_toml_path
//             .expect()
//             .return_once(|| Ok(PathBuf::from("./example_configs/terrain.full.toml")));
//
//         let mock_name = mock_fs::get_terrain_name_context();
//         mock_name.expect().return_const("test-terrain".to_string());
//
//         let session_id = String::from("session_id");
//         let mock_session_id = mock_misc::get_uuid_context();
//         mock_session_id.expect().return_const(session_id.clone());
//
//         let mock_terrain = mock_fs::get_parsed_terrain_context();
//         mock_terrain
//             .expect()
//             .return_once(|| Ok(test_data::terrain_without_biomes()))
//             .times(1);
//
//         let mocket_new = MockUnix::new_context();
//         mocket_new.expect().returning(|| {
//             let request = proto::Request {
//                 session: Some(proto::request::Session::SessionId("session_id".to_string())),
//                 args: Some(proto::request::Args::Activate(proto::ActivateRequest {
//                     terrain_name: "test-terrain".to_string(),
//                     biome_name: "none".to_string(),
//                     toml_path: "./example_configs/terrain.full.toml".to_string(),
//                 })),
//             };
//
//             let mut mocket = MockUnix::default();
//             mocket
//                 .expect_write::<proto::Request>()
//                 .with(eq(request))
//                 .return_once(|_| Ok(()));
//             mocket.expect_read().return_once(|| {
//                 let mut buf = BytesMut::new();
//                 proto::Response {
//                     result: Some(proto::response::Result::Success(proto::response::Success {
//                         body: Some(proto::response::success::Body::Activate(
//                             proto::ActivateResponse {},
//                         )),
//                     })),
//                 }
//                 .encode(&mut buf)?;
//                 Ok(buf.into())
//             });
//             Ok(mocket)
//         });
//
//         let mock_zsh_env = mock_ops::get_zsh_envs_context();
//         mock_zsh_env
//             .expect()
//             .with(eq("none".to_string()))
//             .return_once(|_| Ok(HashMap::<String, String>::new()))
//             .times(1);
//
//         let mut expected = HashMap::<String, String>::new();
//         expected.insert("VAR1".to_string(), "val1".to_string());
//         expected.insert("VAR2".to_string(), "val2".to_string());
//         expected.insert("VAR3".to_string(), "val3".to_string());
//         expected.insert("TERRAINIUM_ENABLED".to_string(), "true".to_string());
//         expected.insert(
//             "TERRAINIUM_TERRAIN_NAME".to_string(),
//             "test-terrain".to_string(),
//         );
//         expected.insert(
//             "TERRAINIUM_TOML_PATH".to_string(),
//             "./example_configs/terrain.full.toml".to_string(),
//         );
//         expected.insert("TERRAINIUM_SELECTED_BIOME".to_string(), "none".to_string());
//         expected.insert(
//             "TERRAINIUM_SESSION_ID".to_string(),
//             "session_id".to_string(),
//         );
//
//         let dev = std::env::var(TERRAINIUM_DEV);
//         if dev.is_ok() && dev.unwrap() == *"true" {
//             let pwd = std::env::current_dir().context("unable to get current_dir")?;
//             let mut terrainium = pwd.clone();
//             terrainium.push("target/debug/terrainium");
//             let mut terrainium_executor = pwd.clone();
//             terrainium_executor.push("target/debug/terrainium_executor");
//
//             expected.insert(
//                 TERRAINIUM_EXECUTABLE_ENV.to_string(),
//                 terrainium.to_string_lossy().to_string(),
//             );
//             expected.insert(
//                 TERRAINIUM_EXECUTOR_ENV.to_string(),
//                 terrainium_executor.to_string_lossy().to_string(),
//             );
//         } else {
//             expected.insert(
//                 TERRAINIUM_EXECUTABLE_ENV.to_string(),
//                 "terrainium".to_string(),
//             );
//             expected.insert(
//                 TERRAINIUM_EXECUTOR_ENV.to_string(),
//                 "terrainium_executor".to_string(),
//             );
//         }
//
//         let mock_spawn = mock_ops::spawn_context();
//         mock_spawn
//             .expect()
//             .withf(move |args, envs| {
//                 let args_eq = *args == vec!["-s"];
//                 let env_eq = *envs.as_ref().unwrap() == expected;
//                 args_eq && env_eq
//             })
//             .return_once(|_, _| Ok(()))
//             .times(1);
//
//         super::handle(None)?;
//
//         // cleanup
//         if let Some(enabled) = enabled {
//             std::env::set_var("TERRAINIUM_ENABLED", enabled)
//         } else {
//             std::env::remove_var("TERRAINIUM_ENABLED")
//         }
//         Ok(())
//     }
//
//     #[test]
//     #[serial]
//     fn returns_error_if_already_enabled() -> Result<()> {
//         let enabled = std::env::var("TERRAINIUM_ENABLED").ok();
//         std::env::set_var("TERRAINIUM_ENABLED", "true");
//
//         let actual = super::handle(None).unwrap_err().to_string();
//
//         assert_eq!("other terrain is already active", actual);
//
//         // cleanup
//         if let Some(enabled) = enabled {
//             std::env::set_var("TERRAINIUM_ENABLED", enabled)
//         } else {
//             std::env::remove_var("TERRAINIUM_ENABLED")
//         }
//         Ok(())
//     }
// }
