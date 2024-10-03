use crate::client::args::BiomeArg;
use crate::client::handlers::background;
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use crate::common::constants::DESTRUCTORS;
use anyhow::Result;

pub async fn handle(context: &mut Context, biome_arg: Option<BiomeArg>) -> Result<()> {
    background::handle(context, DESTRUCTORS, Terrain::merged_destructors, biome_arg).await
}

#[cfg(test)]
mod tests {
    use crate::client::shell::Zsh;
    use crate::client::types::client::MockClient;
    use crate::client::types::context::Context;
    use crate::common::execute::MockRun;
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

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");

        copy("./tests/data/terrain.example.toml", terrain_toml)
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

                let terrain_name = current_dir_path
                    .file_name()
                    .expect("to be present")
                    .to_str()
                    .expect("converted to string")
                    .to_string();

                let expected = ExecuteRequest {
                    terrain_name,
                    biome_name: "example_biome".to_string(),
                    operation: i32::from(Operation::Destructors),
                    commands: vec![Command {
                        exe: "/bin/bash".to_string(),
                        args: vec![
                            "-c".to_string(),
                            "$PWD/tests/scripts/print_num_for_10_sec".to_string(),
                        ],
                        envs,
                    }],
                };

                *actual == Any::from_msg(&expected).expect("to be converted to any")
            })
            .times(1)
            .return_once(move |_| Ok(()));

        mocket.expect_read().with().times(1).return_once(|| {
            Ok(Any::from_msg(&ExecuteResponse {}).expect("to be converted to any"))
        });

        let mut context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockRun::default()),
            Some(mocket),
        );

        super::handle(&mut context, None)
            .await
            .expect("no error to be thrown");
    }

    #[tokio::test]
    async fn destruct_send_message_to_daemon_and_error() {
        let current_dir = tempdir().expect("failed to create tempdir");

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");

        copy("./tests/data/terrain.example.toml", terrain_toml)
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

                let terrain_name = current_dir_path
                    .file_name()
                    .expect("to be present")
                    .to_str()
                    .expect("converted to string")
                    .to_string();

                let expected = ExecuteRequest {
                    terrain_name,
                    biome_name: "example_biome".to_string(),
                    operation: i32::from(Operation::Destructors),
                    commands: vec![Command {
                        exe: "/bin/bash".to_string(),
                        args: vec![
                            "-c".to_string(),
                            "$PWD/tests/scripts/print_num_for_10_sec".to_string(),
                        ],
                        envs,
                    }],
                };

                *actual == Any::from_msg(&expected).expect("to be converted to any")
            })
            .times(1)
            .return_once(move |_| Ok(()));

        mocket.expect_read().with().times(1).return_once(|| {
            Ok(Any::from_msg(&pb::Error {
                error_message: "failed to execute".to_string(),
            })
            .expect("to be converted to any"))
        });

        let mut context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockRun::default()),
            Some(mocket),
        );

        let err = super::handle(&mut context, None)
            .await
            .expect_err("to be thrown");

        assert_eq!(
            err.to_string(),
            "error response from daemon failed to execute"
        );
    }
}
