use crate::client::args::BiomeArg;
#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::environment::Environment;
use crate::client::types::proto::ProtoRequest;
use crate::client::types::terrain::Terrain;
use crate::common::constants::TERRAINIUMD_SOCKET;
use crate::common::types::pb;
use crate::common::utils::timestamp;
use anyhow::{Context as AnyhowContext, Result};
use std::path::PathBuf;

pub async fn handle(
    context: Context,
    biome: BiomeArg,
    terrain: Terrain,
    client: Option<Client>,
) -> Result<()> {
    let environment = Environment::from(&terrain, biome, context.terrain_dir())
        .context("failed to generate environment")?;

    let mut client: Client = if let Some(client) = client {
        client
    } else {
        Client::new(PathBuf::from(TERRAINIUMD_SOCKET)).await?
    };

    let name = environment.name().to_owned();
    let biome = environment.selected_biome().to_owned();
    match construct(context, environment)? {
        None => {
            println!(
                "no background constructors were found for {}({})",
                name, biome
            );
        }
        Some(request) => {
            client.request(ProtoRequest::Execute(request)).await?;
        }
    }

    Ok(())
}

fn construct(context: Context, environment: Environment) -> Result<Option<pb::Execute>> {
    let commands: Vec<pb::Command> = environment
        .constructors()
        .to_proto_commands(environment.envs())
        .context("failed to convert commands")?;

    if commands.is_empty() {
        return Ok(None);
    }

    let (session_id, terrain_dir, _, toml_path, ..) = context.destructure();

    Ok(Some(pb::Execute {
        session_id,
        terrain_name: environment.name().to_string(),
        biome_name: environment.selected_biome().to_string(),
        terrain_dir: terrain_dir.to_string_lossy().to_string(),
        toml_path: toml_path.to_string_lossy().to_string(),
        is_constructor: true,
        timestamp: timestamp(),
        commands,
    }))
}

#[cfg(test)]
mod tests {
    use crate::client::args::BiomeArg;
    use crate::client::test_utils::assertions::client::ExpectClient;
    use crate::client::test_utils::expected_env_vars_example_biome;
    use crate::client::types::client::MockClient;
    use crate::client::types::config::Config;
    use crate::client::types::context::Context;
    use crate::client::types::proto::ProtoRequest;
    use crate::client::types::terrain::Terrain;
    use crate::common::constants::TERRAIN_SESSION_ID;
    use crate::common::execute::MockExecutor;
    use crate::common::types::pb;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    fn expected_request_example_biome(
        session_id: Option<String>,
        terrain_dir: &Path,
    ) -> pb::Execute {
        pb::Execute {
            session_id,
            terrain_name: "terrainium".to_string(),
            biome_name: "example_biome".to_string(),
            terrain_dir: terrain_dir.to_string_lossy().to_string(),
            toml_path: terrain_dir
                .join("terrain.toml")
                .to_string_lossy()
                .to_string(),
            is_constructor: true,
            timestamp: "timestamp".to_string(),
            commands: vec![pb::Command {
                exe: "/bin/bash".to_string(),
                args: vec![
                    "-c".to_string(),
                    "$PWD/tests/scripts/print_num_for_10_sec".to_string(),
                ],
                envs: expected_env_vars_example_biome(terrain_dir),
                cwd: terrain_dir.to_string_lossy().to_string(),
            }],
        }
    }

    #[tokio::test]
    async fn test_construct_sends_request() {
        let session_id = std::env::var(TERRAIN_SESSION_ID).ok();

        let terrain_dir = tempdir().unwrap();
        let context = Context::build(
            terrain_dir.path().to_path_buf(),
            PathBuf::new(),
            terrain_dir.path().join("terrain.toml"),
            Config::default(),
            MockExecutor::new(),
        );

        let client = ExpectClient::to_send(ProtoRequest::Execute(expected_request_example_biome(
            session_id,
            terrain_dir.path(),
        )))
        .successfully();

        super::handle(context, BiomeArg::Default, Terrain::example(), Some(client))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_construct_does_not_send_request_if_no_background() {
        let terrain_dir = tempdir().unwrap();
        let context = Context::build(
            terrain_dir.path().to_path_buf(),
            PathBuf::new(),
            terrain_dir.path().join("terrain.toml"),
            Config::default(),
            MockExecutor::new(),
        );

        // client without any expectation
        let client = MockClient::default();

        // expect no error
        // terrain has no background constructors
        super::handle(context, BiomeArg::None, Terrain::example(), Some(client))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_construct_request_returns_error() {
        let session_id = std::env::var(TERRAIN_SESSION_ID).ok();

        let terrain_dir = tempdir().unwrap();
        let context = Context::build(
            terrain_dir.path().to_path_buf(),
            PathBuf::new(),
            terrain_dir.path().join("terrain.toml"),
            Config::default(),
            MockExecutor::new(),
        );

        let expected_error = "failed to parse execute request".to_string();

        let client = ExpectClient::to_send(ProtoRequest::Execute(expected_request_example_biome(
            session_id,
            terrain_dir.path(),
        )))
        .with_returning_error(expected_error.clone());

        let actual_error =
            super::handle(context, BiomeArg::Default, Terrain::example(), Some(client))
                .await
                .expect_err("expected error")
                .to_string();

        assert_eq!(actual_error, expected_error);
    }
}
