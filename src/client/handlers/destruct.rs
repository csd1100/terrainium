use crate::client::args::BiomeArg;
use crate::client::handlers::background;
#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use crate::common::utils::timestamp;
use anyhow::Result;

pub async fn handle(
    context: Context,
    biome: BiomeArg,
    terrain: Terrain,
    client: Option<Client>,
) -> Result<()> {
    background::handle(context, biome, terrain, false, timestamp(), client).await
}

#[cfg(test)]
mod tests {
    use crate::client::args::BiomeArg;
    use crate::client::test_utils::assertions::client::ExpectClient;
    use crate::client::test_utils::expected_execute_request_example_biome;
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

    pub(crate) fn expected_request_destruct_example_biome(
        session_id: Option<String>,
        terrain_dir: &Path,
    ) -> pb::Execute {
        expected_execute_request_example_biome(session_id, terrain_dir, false)
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

        let client = ExpectClient::to_send(ProtoRequest::Execute(
            expected_request_destruct_example_biome(session_id, terrain_dir.path()),
        ))
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

        let client = ExpectClient::to_send(ProtoRequest::Execute(
            expected_request_destruct_example_biome(session_id, terrain_dir.path()),
        ))
        .with_returning_error(expected_error.clone());

        let actual_error =
            super::handle(context, BiomeArg::Default, Terrain::example(), Some(client))
                .await
                .expect_err("expected error")
                .to_string();

        assert_eq!(actual_error, expected_error);
    }
}
