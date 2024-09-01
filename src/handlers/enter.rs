use std::collections::BTreeMap;

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
        operations::{get_current_dir_toml, get_parsed_terrain, get_terrain_name, merge_maps},
    },
    proto::{self, ActivateRequest},
    types::args::BiomeArg,
};

#[double]
use crate::helpers::utils::misc;
use crate::helpers::utils::Paths;
#[double]
use crate::types::socket::Unix;

#[double]
use crate::shell::zsh::ops;

fn add_executables(map: &mut BTreeMap<String, String>, paths: &Paths) -> Result<()> {
    let dev = std::env::var(TERRAINIUM_DEV);
    if dev.is_ok() && dev? == *"true" {
        let mut terrainium = paths.get_cwd().clone();
        terrainium.push("target/debug/terrainium");
        let mut terrainium_executor = paths.get_cwd().clone();
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

pub fn handle(biome: Option<BiomeArg>, paths: &Paths) -> Result<()> {
    let enabled = std::env::var(TERRAINIUM_ENABLED);
    if enabled.is_ok() && enabled? == *"true" {
        return Err(anyhow!("other terrain is already active"));
    }

    let terrain = get_parsed_terrain(paths)?;

    let terrain_name: String = get_terrain_name(paths.get_cwd());
    let toml_path = get_current_dir_toml(paths)?.to_string_lossy().to_string();
    let session_id = misc::get_uuid();
    let biome_name = terrain.get_selected_biome_name(&biome)?;

    let mut envs = BTreeMap::<String, String>::new();
    envs.insert(TERRAINIUM_ENABLED.to_string(), "true".to_string());
    envs.insert(TERRAINIUM_TOML_PATH.to_string(), toml_path.clone());
    envs.insert(TERRAINIUM_TERRAIN_NAME.to_string(), terrain_name.clone());
    envs.insert(TERRAINIUM_SESSION_ID.to_string(), session_id.clone());
    envs.insert(TERRAINIUM_SELECTED_BIOME.to_string(), biome_name.clone());
    add_executables(&mut envs, paths)?;

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

    let zsh_env = ops::get_zsh_envs(terrain.get_selected_biome_name(&biome)?, paths)
        .context("unable to set zsh environment variables")?;
    let mut merged = merge_maps(&envs.clone(), &zsh_env);

    let selected = terrain
        .get(biome.clone())
        .context("unable to select biome")?;
    merged = merge_maps(
        &merged,
        &selected.env.unwrap_or(BTreeMap::<String, String>::new()),
    );
    ops::spawn(vec!["-s"], Some(merged)).context("unable to start zsh")?;

    Ok(())
}

#[cfg(test)]
mod test {
    use std::{collections::BTreeMap, path::PathBuf};

    use crate::helpers::utils::get_paths;
    use crate::types::args::BiomeArg;
    use crate::{
        helpers::{
            constants::{TERRAINIUM_DEV, TERRAINIUM_EXECUTABLE_ENV, TERRAINIUM_EXECUTOR_ENV},
            utils::mock_misc,
        },
        proto,
        shell::zsh::mock_ops,
        types::socket::MockUnix,
    };
    use anyhow::{Context, Result};
    use mockall::predicate::eq;
    use prost::{bytes::BytesMut, Message};
    use serial_test::serial;
    use tempfile::tempdir;

    fn get_terrain_name_from_cwd(cwd: &PathBuf) -> String {
        cwd.file_name().unwrap().to_str().unwrap().to_string()
    }

    fn get_terrain_toml_from_cwd(cwd: &PathBuf) -> String {
        let mut toml_path: PathBuf = cwd.into();
        toml_path.push("terrain.toml");
        toml_path.to_str().unwrap().to_string()
    }

    #[test]
    #[serial]
    fn enter_enters_default() -> Result<()> {
        // setup terrain.toml
        let test_dir = tempdir()?;
        let home_dir = tempdir()?;
        let test_dir_path: PathBuf = test_dir.path().into();
        let home_dir_path: PathBuf = home_dir.path().into();

        let paths = get_paths(home_dir_path, test_dir_path)?;

        let mut terrain_toml_path: PathBuf = test_dir.path().into();
        terrain_toml_path.push("terrain.toml");
        std::fs::copy("./tests/data/terrain.full.toml", &terrain_toml_path)?;

        // unset TERRAINIUM_ENABLED
        let enabled = std::env::var("TERRAINIUM_ENABLED").ok();
        std::env::remove_var("TERRAINIUM_ENABLED");

        // setup mock session id
        let session_id = String::from("session_id");
        let mock_session_id = mock_misc::get_uuid_context();
        mock_session_id.expect().return_const(session_id.clone());

        // mock daemon interactions
        let test_dir_path: PathBuf = test_dir.path().into();
        let mocket_new = MockUnix::new_context();
        mocket_new.expect().returning(move || {
            let request = proto::Request {
                session: Some(proto::request::Session::SessionId("session_id".to_string())),
                args: Some(proto::request::Args::Activate(proto::ActivateRequest {
                    terrain_name: get_terrain_name_from_cwd(&test_dir_path),
                    biome_name: "example_biome".to_string(),
                    toml_path: get_terrain_toml_from_cwd(&test_dir_path),
                })),
            };
            let mut mocket = MockUnix::default();
            mocket
                .expect_write::<proto::Request>()
                .with(eq(request))
                .return_once(|_| Ok(()));
            mocket.expect_read().return_once(|| {
                let mut buf = BytesMut::new();
                proto::Response {
                    result: Some(proto::response::Result::Success(proto::response::Success {
                        body: Some(proto::response::success::Body::Activate(
                            proto::ActivateResponse {},
                        )),
                    })),
                }
                .encode(&mut buf)?;
                Ok(buf.into())
            });
            Ok(mocket)
        });

        // mock environment variables
        let mock_zsh_env = mock_ops::get_zsh_envs_context();
        mock_zsh_env
            .expect()
            .with(eq("example_biome".to_string()), eq(paths.clone()))
            .return_once(|_, _| Ok(BTreeMap::<String, String>::new()))
            .times(1);

        // expected environment variables to spawn background processes
        let mut expected = BTreeMap::<String, String>::new();
        expected.insert("EDITOR".to_string(), "nvim".to_string());
        expected.insert("TEST".to_string(), "value".to_string());
        expected.insert("TERRAINIUM_ENABLED".to_string(), "true".to_string());
        expected.insert(
            "TERRAINIUM_TERRAIN_NAME".to_string(),
            get_terrain_name_from_cwd(paths.get_cwd()),
        );
        expected.insert(
            "TERRAINIUM_TOML_PATH".to_string(),
            get_terrain_toml_from_cwd(paths.get_cwd()),
        );
        expected.insert(
            "TERRAINIUM_SELECTED_BIOME".to_string(),
            "example_biome".to_string(),
        );
        expected.insert("TERRAINIUM_SESSION_ID".to_string(), session_id);

        let dev = std::env::var(TERRAINIUM_DEV);
        if dev.is_ok() && dev? == *"true" {
            let pwd = std::env::current_dir().context("unable to get current_dir")?;
            let mut terrainium = pwd.clone();
            terrainium.push("target/debug/terrainium");
            let mut terrainium_executor = pwd.clone();
            terrainium_executor.push("target/debug/terrainium_executor");

            expected.insert(
                TERRAINIUM_EXECUTABLE_ENV.to_string(),
                terrainium.to_string_lossy().to_string(),
            );
            expected.insert(
                TERRAINIUM_EXECUTOR_ENV.to_string(),
                terrainium_executor.to_string_lossy().to_string(),
            );
        } else {
            expected.insert(
                TERRAINIUM_EXECUTABLE_ENV.to_string(),
                "terrainium".to_string(),
            );
            expected.insert(
                TERRAINIUM_EXECUTOR_ENV.to_string(),
                "terrainium_executor".to_string(),
            );
        }

        // mock and validate spawn call
        let mock_spawn = mock_ops::spawn_context();
        mock_spawn
            .expect()
            .withf(move |args, envs| {
                let args_eq = *args == vec!["-s"];
                let env_eq = *envs.as_ref().unwrap() == expected;
                args_eq && env_eq
            })
            .return_once(|_, _| Ok(()))
            .times(1);

        super::handle(None, &paths)?;

        // cleanup
        if let Some(enabled) = enabled {
            std::env::set_var("TERRAINIUM_ENABLED", enabled)
        } else {
            std::env::remove_var("TERRAINIUM_ENABLED")
        }
        Ok(())
    }

    #[test]
    #[serial]
    fn enter_enters_selected() -> Result<()> {
        // setup terrain.toml
        let test_dir = tempdir()?;
        let home_dir = tempdir()?;
        let test_dir_path: PathBuf = test_dir.path().into();
        let home_dir_path: PathBuf = home_dir.path().into();

        let paths = get_paths(home_dir_path, test_dir_path)?;

        let mut terrain_toml_path: PathBuf = test_dir.path().into();
        terrain_toml_path.push("terrain.toml");
        std::fs::copy("./tests/data/terrain.full.toml", &terrain_toml_path)?;

        // unset TERRAINIUM_ENABLED
        let enabled = std::env::var("TERRAINIUM_ENABLED").ok();
        std::env::remove_var("TERRAINIUM_ENABLED");

        let session_id = String::from("session_id");
        let mock_session_id = mock_misc::get_uuid_context();
        mock_session_id.expect().return_const(session_id.clone());

        let test_dir_path: PathBuf = test_dir.path().into();
        let mocket_new = MockUnix::new_context();
        mocket_new.expect().returning(move || {
            let request = proto::Request {
                session: Some(proto::request::Session::SessionId("session_id".to_string())),
                args: Some(proto::request::Args::Activate(proto::ActivateRequest {
                    terrain_name: get_terrain_name_from_cwd(&test_dir_path),
                    biome_name: "example_biome2".to_string(),
                    toml_path: get_terrain_toml_from_cwd(&test_dir_path),
                })),
            };
            let mut mocket = MockUnix::default();
            mocket
                .expect_write::<proto::Request>()
                .with(eq(request))
                .return_once(|_| Ok(()));
            mocket.expect_read().return_once(|| {
                let mut buf = BytesMut::new();
                proto::Response {
                    result: Some(proto::response::Result::Success(proto::response::Success {
                        body: Some(proto::response::success::Body::Activate(
                            proto::ActivateResponse {},
                        )),
                    })),
                }
                .encode(&mut buf)?;
                Ok(buf.into())
            });
            Ok(mocket)
        });

        let mock_zsh_env = mock_ops::get_zsh_envs_context();
        mock_zsh_env
            .expect()
            .with(eq("example_biome2".to_string()), eq(paths.clone()))
            .return_once(|_, _| Ok(BTreeMap::<String, String>::new()))
            .times(1);

        let mut expected = BTreeMap::<String, String>::new();
        expected.insert("EDITOR".to_string(), "nano".to_string());
        expected.insert("TEST".to_string(), "value".to_string());
        expected.insert("TERRAINIUM_ENABLED".to_string(), "true".to_string());
        expected.insert(
            "TERRAINIUM_TERRAIN_NAME".to_string(),
            get_terrain_name_from_cwd(paths.get_cwd()),
        );
        expected.insert(
            "TERRAINIUM_TOML_PATH".to_string(),
            get_terrain_toml_from_cwd(paths.get_cwd()),
        );
        expected.insert(
            "TERRAINIUM_SELECTED_BIOME".to_string(),
            "example_biome2".to_string(),
        );
        expected.insert(
            "TERRAINIUM_SESSION_ID".to_string(),
            "session_id".to_string(),
        );

        let dev = std::env::var(TERRAINIUM_DEV);
        if dev.is_ok() && dev.unwrap() == *"true" {
            let pwd = std::env::current_dir().context("unable to get current_dir")?;
            let mut terrainium = pwd.clone();
            terrainium.push("target/debug/terrainium");
            let mut terrainium_executor = pwd.clone();
            terrainium_executor.push("target/debug/terrainium_executor");

            expected.insert(
                TERRAINIUM_EXECUTABLE_ENV.to_string(),
                terrainium.to_string_lossy().to_string(),
            );
            expected.insert(
                TERRAINIUM_EXECUTOR_ENV.to_string(),
                terrainium_executor.to_string_lossy().to_string(),
            );
        } else {
            expected.insert(
                TERRAINIUM_EXECUTABLE_ENV.to_string(),
                "terrainium".to_string(),
            );
            expected.insert(
                TERRAINIUM_EXECUTOR_ENV.to_string(),
                "terrainium_executor".to_string(),
            );
        }

        let mock_spawn = mock_ops::spawn_context();
        mock_spawn
            .expect()
            .withf(move |args, envs| {
                let args_eq = *args == vec!["-s"];
                let env_eq = *envs.as_ref().unwrap() == expected;
                args_eq && env_eq
            })
            .return_once(|_, _| Ok(()))
            .times(1);

        super::handle(Some(BiomeArg::Value("example_biome2".to_string())), &paths)?;

        // cleanup
        if let Some(enabled) = enabled {
            std::env::set_var("TERRAINIUM_ENABLED", enabled)
        } else {
            std::env::remove_var("TERRAINIUM_ENABLED")
        }
        Ok(())
    }

    #[test]
    #[serial]
    fn enter_enters_main() -> Result<()> {
        // setup terrain.toml
        let test_dir = tempdir()?;
        let home_dir = tempdir()?;
        let test_dir_path: PathBuf = test_dir.path().into();
        let home_dir_path: PathBuf = home_dir.path().into();

        let paths = get_paths(home_dir_path, test_dir_path)?;

        let mut terrain_toml_path: PathBuf = test_dir.path().into();
        terrain_toml_path.push("terrain.toml");
        std::fs::copy(
            "./tests/data/terrain.without.biomes.toml",
            &terrain_toml_path,
        )?;

        // unset TERRAINIUM_ENABLED
        let enabled = std::env::var("TERRAINIUM_ENABLED").ok();
        std::env::remove_var("TERRAINIUM_ENABLED");

        // mock session id
        let session_id = String::from("session_id");
        let mock_session_id = mock_misc::get_uuid_context();
        mock_session_id.expect().return_const(session_id.clone());

        let test_dir_path: PathBuf = test_dir.path().into();
        let mocket_new = MockUnix::new_context();
        mocket_new.expect().returning(move || {
            let request = proto::Request {
                session: Some(proto::request::Session::SessionId("session_id".to_string())),
                args: Some(proto::request::Args::Activate(proto::ActivateRequest {
                    terrain_name: get_terrain_name_from_cwd(&test_dir_path),
                    biome_name: "none".to_string(),
                    toml_path: get_terrain_toml_from_cwd(&test_dir_path),
                })),
            };

            let mut mocket = MockUnix::default();
            mocket
                .expect_write::<proto::Request>()
                .with(eq(request))
                .return_once(|_| Ok(()));
            mocket.expect_read().return_once(|| {
                let mut buf = BytesMut::new();
                proto::Response {
                    result: Some(proto::response::Result::Success(proto::response::Success {
                        body: Some(proto::response::success::Body::Activate(
                            proto::ActivateResponse {},
                        )),
                    })),
                }
                .encode(&mut buf)?;
                Ok(buf.into())
            });
            Ok(mocket)
        });

        let mock_zsh_env = mock_ops::get_zsh_envs_context();
        mock_zsh_env
            .expect()
            .with(eq("none".to_string()), eq(paths.clone()))
            .return_once(|_, _| Ok(BTreeMap::<String, String>::new()))
            .times(1);

        let mut expected = BTreeMap::<String, String>::new();
        expected.insert("VAR1".to_string(), "val1".to_string());
        expected.insert("VAR2".to_string(), "val2".to_string());
        expected.insert("VAR3".to_string(), "val3".to_string());
        expected.insert("TERRAINIUM_ENABLED".to_string(), "true".to_string());
        expected.insert(
            "TERRAINIUM_TERRAIN_NAME".to_string(),
            get_terrain_name_from_cwd(paths.get_cwd()),
        );
        expected.insert(
            "TERRAINIUM_TOML_PATH".to_string(),
            get_terrain_toml_from_cwd(paths.get_cwd()),
        );
        expected.insert("TERRAINIUM_SELECTED_BIOME".to_string(), "none".to_string());
        expected.insert(
            "TERRAINIUM_SESSION_ID".to_string(),
            "session_id".to_string(),
        );

        let dev = std::env::var(TERRAINIUM_DEV);
        if dev.is_ok() && dev.unwrap() == *"true" {
            let pwd = std::env::current_dir().context("unable to get current_dir")?;
            let mut terrainium = pwd.clone();
            terrainium.push("target/debug/terrainium");
            let mut terrainium_executor = pwd.clone();
            terrainium_executor.push("target/debug/terrainium_executor");

            expected.insert(
                TERRAINIUM_EXECUTABLE_ENV.to_string(),
                terrainium.to_string_lossy().to_string(),
            );
            expected.insert(
                TERRAINIUM_EXECUTOR_ENV.to_string(),
                terrainium_executor.to_string_lossy().to_string(),
            );
        } else {
            expected.insert(
                TERRAINIUM_EXECUTABLE_ENV.to_string(),
                "terrainium".to_string(),
            );
            expected.insert(
                TERRAINIUM_EXECUTOR_ENV.to_string(),
                "terrainium_executor".to_string(),
            );
        }

        let mock_spawn = mock_ops::spawn_context();
        mock_spawn
            .expect()
            .withf(move |args, envs| {
                let args_eq = *args == vec!["-s"];
                let env_eq = *envs.as_ref().unwrap() == expected;
                args_eq && env_eq
            })
            .return_once(|_, _| Ok(()))
            .times(1);

        super::handle(None, &paths)?;

        // cleanup
        if let Some(enabled) = enabled {
            std::env::set_var("TERRAINIUM_ENABLED", enabled)
        } else {
            std::env::remove_var("TERRAINIUM_ENABLED")
        }
        Ok(())
    }

    #[test]
    #[serial]
    fn returns_error_if_already_enabled() -> Result<()> {
        let test_dir = tempdir()?;
        let home_dir = tempdir()?;
        let test_dir_path: PathBuf = test_dir.path().into();
        let home_dir_path: PathBuf = home_dir.path().into();

        let paths = get_paths(home_dir_path, test_dir_path)?;

        let enabled = std::env::var("TERRAINIUM_ENABLED").ok();
        std::env::set_var("TERRAINIUM_ENABLED", "true");

        let actual = super::handle(None, &paths).unwrap_err().to_string();

        assert_eq!("other terrain is already active", actual);

        // cleanup
        if let Some(enabled) = enabled {
            std::env::set_var("TERRAINIUM_ENABLED", enabled)
        } else {
            std::env::remove_var("TERRAINIUM_ENABLED")
        }
        Ok(())
    }
}
