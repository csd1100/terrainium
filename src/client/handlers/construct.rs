use crate::client::args::{option_string_from, BiomeArg};
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use crate::common::types::pb;
use crate::common::types::pb::{Error, ExecuteRequest, ExecuteResponse};
use crate::common::types::socket::Socket;
use anyhow::{anyhow, Context as AnyhowContext, Result};
use prost_types::Any;
use std::fs::read_to_string;

pub async fn handle(context: &mut Context, biome_arg: Option<BiomeArg>) -> Result<()> {
    let terrain = Terrain::from_toml(
        read_to_string(context.toml_path()?).context("failed to read terrain.toml")?,
    )
    .expect("terrain to be parsed from toml");

    let constructors = terrain
        .merged_constructors(&option_string_from(&biome_arg))
        .context("failed to merge constructors")?;

    let mut envs = terrain
        .merged_envs(&option_string_from(&biome_arg))
        .context("failed to merge envs")?;

    let mut final_envs = context.terrainium_envs().clone();
    final_envs.append(&mut envs);

    let commands: Vec<pb::Command> = constructors
        .background()
        .iter()
        .map(|command| {
            let mut command: pb::Command = command.clone().into();
            command.envs = final_envs.clone();
            command
        })
        .collect();

    let request = ExecuteRequest {
        terrain_name: context.name(),
        operation: i32::from(pb::Operation::Constructors),
        commands,
    };

    let client = context.socket();

    client
        .write_and_stop(Any::from_msg(&request).unwrap())
        .await?;

    let response: Any = client.read().await?;
    let execute_response: Result<ExecuteResponse> =
        Any::to_msg(&response).context("failed to convert to execute response from Any");

    if execute_response.is_ok() {
        println!("Success");
    } else {
        let error: Error = Any::to_msg(&response).context("failed to convert to error from Any")?;
        return Err(anyhow!(
            "error response from daemon {}",
            error.error_message
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::client::types::client::MockClient;
    use crate::client::types::context::Context;
    use crate::common::execute::MockRun;
    use crate::common::shell::Zsh;
    use crate::common::types::pb;
    use crate::common::types::pb::{Command, ExecuteRequest, ExecuteResponse, Operation};
    use prost_types::Any;
    use std::collections::BTreeMap;
    use std::fs::copy;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[tokio::test]
    async fn construct_send_message_to_daemon() {
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
                    operation: i32::from(Operation::Constructors),
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
    async fn construct_send_message_to_daemon_and_error() {
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
                    operation: i32::from(Operation::Constructors),
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
