use crate::client::args::BiomeArg;
use crate::client::handlers::background;
use crate::client::types::context::Context;
use crate::common::constants::{DESTRUCTORS, TERRAIN_AUTO_APPLY, TERRAIN_SELECTED_BIOME};
use anyhow::{anyhow, Context as AnyhowContext, Result};
use std::collections::BTreeMap;
use std::env;

pub async fn handle(context: Context) -> Result<()> {
    let session_id = context.session_id();
    let selected_biome = env::var(TERRAIN_SELECTED_BIOME).unwrap_or_default();

    if session_id.is_empty() || selected_biome.is_empty() {
        return Err(anyhow!(
            "no active terrain found, use `terrainium enter` command to activate a terrain."
        ));
    }

    if should_run_destructor() {
        background::handle(
            &context,
            DESTRUCTORS,
            Some(BiomeArg::Some(selected_biome)),
            Some(BTreeMap::<String, String>::new()),
            None,
        )
        .await
        .context("failed to run destructors")?;
    }

    Ok(())
}

/// `terrainium exit` should run background destructor commands only in following case:
/// 1. Auto-apply is disabled
/// 2. Auto-apply is enabled but background flag is also turned on
fn should_run_destructor() -> bool {
    let auto_apply = env::var(TERRAIN_AUTO_APPLY);
    match auto_apply {
        Ok(auto_apply) => auto_apply == "all" || auto_apply == "background",
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use crate::client::shell::Zsh;
    use crate::client::types::client::MockClient;
    use crate::client::types::context::Context;
    use crate::common::constants::{
        TERRAINIUMD_SOCKET, TERRAINIUM_EXECUTABLE, TERRAIN_AUTO_APPLY, TERRAIN_SELECTED_BIOME,
    };
    use crate::common::execute::test::{restore_env_var, set_env_var};
    use crate::common::execute::MockCommandToRun;
    use crate::common::types::pb;
    use crate::common::types::pb::{Command, ExecuteRequest, ExecuteResponse, Operation};
    use prost_types::Any;
    use serial_test::serial;
    use std::collections::BTreeMap;
    use std::fs::copy;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[serial]
    #[tokio::test]
    async fn exit_send_message_to_daemon() {
        let orig_selected_biome = set_env_var(
            TERRAIN_SELECTED_BIOME.to_string(),
            "example_biome".to_string(),
        );

        let current_dir = tempdir().expect("failed to create tempdir");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("copy to terrain to test dir");

        let current_dir_path: PathBuf = current_dir.path().into();
        let mut mocket = MockClient::default();
        mocket
            .expect_write_and_stop()
            .withf(move |actual: &Any| {
                let mut envs: BTreeMap<String, String> = BTreeMap::new();
                envs.insert("EDITOR".to_string(), "nvim".to_string());
                envs.insert("PAGER".to_string(), "less".to_string());
                envs.insert(
                    "TERRAIN_DIR".to_string(),
                    current_dir_path.to_str().unwrap().to_string(),
                );
                envs.insert("TERRAIN_ENABLED".to_string(), "true".to_string());
                envs.insert("TERRAIN_SESSION_ID".to_string(), "some".to_string());

                let exe = std::env::args().next().unwrap();
                envs.insert(TERRAINIUM_EXECUTABLE.to_string(), exe);

                let terrain_name = current_dir_path
                    .file_name()
                    .expect("to be present")
                    .to_str()
                    .expect("converted to string")
                    .to_string();

                let commands = vec![Command {
                    exe: "/bin/bash".to_string(),
                    args: vec![
                        "-c".to_string(),
                        "$PWD/tests/scripts/print_num_for_10_sec".to_string(),
                    ],
                    envs,
                }];

                let actual: ExecuteRequest =
                    Any::to_msg(actual).expect("failed to convert to Activate request");

                actual.terrain_name == terrain_name
                    && !actual.session_id.is_empty()
                    && actual.biome_name == "example_biome"
                    && actual.toml_path == terrain_toml.display().to_string()
                    && actual.is_activate
                    && actual.commands == commands
                    && actual.operation == i32::from(Operation::Destructors)
            })
            .times(1)
            .return_once(move |_| Ok(()));

        mocket.expect_read().with().times(1).return_once(|| {
            Ok(Any::from_msg(&ExecuteResponse {}).expect("to be converted to any"))
        });

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
        );

        let new_client = MockClient::new_context();
        new_client
            .expect()
            .withf(|path| path.to_str() == Some(TERRAINIUMD_SOCKET))
            .return_once(|_| Ok(mocket))
            .times(1);

        super::handle(context).await.expect("no error to be thrown");

        restore_env_var(TERRAIN_SELECTED_BIOME.to_string(), orig_selected_biome);
    }

    #[serial]
    #[tokio::test]
    async fn exit_send_message_to_daemon_and_error() {
        let orig_selected_biome = set_env_var(
            TERRAIN_SELECTED_BIOME.to_string(),
            "example_biome".to_string(),
        );
        let current_dir = tempdir().expect("failed to create tempdir");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("copy to terrain to test dir");

        let current_dir_path: PathBuf = current_dir.path().into();
        let mut mocket = MockClient::default();
        mocket
            .expect_write_and_stop()
            .withf(move |actual: &Any| {
                let mut envs: BTreeMap<String, String> = BTreeMap::new();
                envs.insert("EDITOR".to_string(), "nvim".to_string());
                envs.insert("PAGER".to_string(), "less".to_string());
                envs.insert(
                    "TERRAIN_DIR".to_string(),
                    current_dir_path.to_str().unwrap().to_string(),
                );
                envs.insert("TERRAIN_ENABLED".to_string(), "true".to_string());
                envs.insert("TERRAIN_SESSION_ID".to_string(), "some".to_string());

                let exe = std::env::args().next().unwrap();
                envs.insert(TERRAINIUM_EXECUTABLE.to_string(), exe);

                let terrain_name = current_dir_path
                    .file_name()
                    .expect("to be present")
                    .to_str()
                    .expect("converted to string")
                    .to_string();

                let commands = vec![Command {
                    exe: "/bin/bash".to_string(),
                    args: vec![
                        "-c".to_string(),
                        "$PWD/tests/scripts/print_num_for_10_sec".to_string(),
                    ],
                    envs,
                }];

                let actual: ExecuteRequest =
                    Any::to_msg(actual).expect("failed to convert to Activate request");

                actual.terrain_name == terrain_name
                    && !actual.session_id.is_empty()
                    && actual.biome_name == "example_biome"
                    && actual.toml_path == terrain_toml.display().to_string()
                    && actual.is_activate
                    && actual.commands == commands
                    && actual.operation == i32::from(Operation::Destructors)
            })
            .times(1)
            .return_once(move |_| Ok(()));

        mocket.expect_read().with().times(1).return_once(|| {
            Ok(Any::from_msg(&pb::Error {
                error_message: "failed to execute".to_string(),
            })
            .expect("to be converted to any"))
        });

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
        );

        let new_client = MockClient::new_context();
        new_client
            .expect()
            .withf(|path| path.to_str() == Some(TERRAINIUMD_SOCKET))
            .return_once(|_| Ok(mocket))
            .times(1);

        let err = super::handle(context).await.expect_err("to be thrown");

        assert_eq!(err.to_string(), "failed to run destructors");

        restore_env_var(TERRAIN_SELECTED_BIOME.to_string(), orig_selected_biome);
    }

    #[serial]
    #[tokio::test]
    async fn exit_does_not_send_message_to_daemon_auto_apply_without_background() {
        let orig_selected_biome = set_env_var(
            TERRAIN_SELECTED_BIOME.to_string(),
            "example_biome".to_string(),
        );
        let orig_auto_apply = set_env_var(TERRAIN_AUTO_APPLY.to_string(), "enabled".to_string());

        let current_dir = tempdir().expect("failed to create tempdir");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy(
            "./tests/data/terrain.example.auto_apply.enabled.toml",
            &terrain_toml,
        )
        .expect("copy to terrain to test dir");

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
        );

        super::handle(context).await.expect("no error to be thrown");

        restore_env_var(TERRAIN_SELECTED_BIOME.to_string(), orig_selected_biome);
        restore_env_var(TERRAIN_AUTO_APPLY.to_string(), orig_auto_apply);
    }

    #[serial]
    #[tokio::test]
    async fn exit_does_send_message_to_daemon_auto_apply() {
        let orig_selected_biome = set_env_var(
            TERRAIN_SELECTED_BIOME.to_string(),
            "example_biome".to_string(),
        );
        let orig_auto_apply = set_env_var(TERRAIN_AUTO_APPLY.to_string(), "all".to_string());

        let current_dir = tempdir().expect("failed to create tempdir");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("copy to terrain to test dir");

        let current_dir_path: PathBuf = current_dir.path().into();
        let mut mocket = MockClient::default();
        mocket
            .expect_write_and_stop()
            .withf(move |actual: &Any| {
                let mut envs: BTreeMap<String, String> = BTreeMap::new();
                envs.insert("EDITOR".to_string(), "nvim".to_string());
                envs.insert("PAGER".to_string(), "less".to_string());
                envs.insert(
                    "TERRAIN_DIR".to_string(),
                    current_dir_path.to_str().unwrap().to_string(),
                );
                envs.insert("TERRAIN_ENABLED".to_string(), "true".to_string());
                envs.insert("TERRAIN_SESSION_ID".to_string(), "some".to_string());

                let exe = std::env::args().next().unwrap();
                envs.insert(TERRAINIUM_EXECUTABLE.to_string(), exe);

                let terrain_name = current_dir_path
                    .file_name()
                    .expect("to be present")
                    .to_str()
                    .expect("converted to string")
                    .to_string();

                let commands = vec![Command {
                    exe: "/bin/bash".to_string(),
                    args: vec![
                        "-c".to_string(),
                        "$PWD/tests/scripts/print_num_for_10_sec".to_string(),
                    ],
                    envs,
                }];

                let actual: ExecuteRequest =
                    Any::to_msg(actual).expect("failed to convert to Activate request");

                actual.terrain_name == terrain_name
                    && !actual.session_id.is_empty()
                    && actual.biome_name == "example_biome"
                    && actual.toml_path == terrain_toml.display().to_string()
                    && actual.is_activate
                    && actual.commands == commands
                    && actual.operation == i32::from(Operation::Destructors)
            })
            .times(1)
            .return_once(move |_| Ok(()));

        mocket.expect_read().with().times(1).return_once(|| {
            Ok(Any::from_msg(&ExecuteResponse {}).expect("to be converted to any"))
        });

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
        );

        let new_client = MockClient::new_context();
        new_client
            .expect()
            .withf(|path| path.to_str() == Some(TERRAINIUMD_SOCKET))
            .return_once(|_| Ok(mocket))
            .times(1);

        super::handle(context).await.expect("no error to be thrown");

        restore_env_var(TERRAIN_SELECTED_BIOME.to_string(), orig_selected_biome);
        restore_env_var(TERRAIN_AUTO_APPLY.to_string(), orig_auto_apply);
    }
}
