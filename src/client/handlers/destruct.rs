use crate::client::args::BiomeArg;
use crate::client::handlers::background;
#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::common::constants::DESTRUCTORS;
use anyhow::Result;

pub async fn handle(
    context: Context,
    biome_arg: Option<BiomeArg>,
    client: Option<Client>,
) -> Result<()> {
    if let Some(client) = client {
        background::handle(&context, client, DESTRUCTORS, biome_arg, None).await?
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::client::shell::Zsh;
    use crate::client::types::client::MockClient;
    use crate::client::types::context::Context;
    use crate::common::constants::TERRAINIUM_EXECUTABLE;
    use crate::common::execute::MockCommandToRun;
    use crate::common::types::pb;
    use crate::common::types::pb::{Command, ExecuteRequest, ExecuteResponse, Operation};
    use prost_types::Any;
    use std::collections::BTreeMap;
    use std::fs::copy;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[tokio::test]
    async fn destruct_send_message_to_daemon() {
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
                    && actual.session_id.is_empty()
                    && actual.biome_name == "example_biome"
                    && actual.toml_path == terrain_toml.display().to_string()
                    && !actual.is_activate
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

        super::handle(context, None, Some(mocket))
            .await
            .expect("no error to be thrown");
    }

    #[tokio::test]
    async fn destruct_send_message_to_daemon_and_error() {
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
                    && actual.session_id.is_empty()
                    && actual.biome_name == "example_biome"
                    && actual.toml_path == terrain_toml.display().to_string()
                    && !actual.is_activate
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

        let err = super::handle(context, None, Some(mocket))
            .await
            .expect_err("to be thrown");

        assert_eq!(
            err.to_string(),
            "error response from daemon failed to execute"
        );
    }
}
