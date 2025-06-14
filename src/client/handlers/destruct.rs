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
    use crate::client::types::client::MockClient;
    use crate::client::types::config::Config;
    use crate::client::types::context::Context;
    use crate::client::types::proto::ProtoRequest;
    use crate::client::types::terrain::Terrain;
    use crate::common::constants::{TERRAIN_SESSION_ID, TERRAIN_TOML};
    use crate::common::execute::MockExecutor;
    use crate::common::test_utils::expected_execute_request_example_biome;
    use crate::common::test_utils::TEST_TERRAIN_DIR;
    use crate::common::types::pb;
    use std::path::PathBuf;

    pub(crate) fn expected_request_destruct_example_biome(
        session_id: Option<String>,
    ) -> pb::Execute {
        expected_execute_request_example_biome(session_id, false)
    }

    #[tokio::test]
    async fn test_destruct_sends_request() {
        let session_id = std::env::var(TERRAIN_SESSION_ID).ok();

        let terrain_dir = PathBuf::from(TEST_TERRAIN_DIR);
        let toml_path = terrain_dir.join(TERRAIN_TOML);

        let mut context = Context::build(
            terrain_dir,
            PathBuf::new(),
            toml_path,
            Config::default(),
            MockExecutor::new(),
        );

        if let Some(session_id) = &session_id {
            context = context.set_session_id(session_id.clone());
        }

        let client = ExpectClient::to_send(ProtoRequest::Execute(
            expected_request_destruct_example_biome(session_id),
        ))
        .successfully();

        super::handle(context, BiomeArg::Default, Terrain::example(), Some(client))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_destruct_does_not_send_request_if_no_background() {
        let terrain_dir = PathBuf::from(TEST_TERRAIN_DIR);
        let toml_path = terrain_dir.join(TERRAIN_TOML);

        let context = Context::build(
            terrain_dir,
            PathBuf::new(),
            toml_path,
            Config::default(),
            MockExecutor::new(),
        );

        // client without any expectation
        let client = MockClient::default();

        // expect no error
        // terrain has no background destructors
        super::handle(context, BiomeArg::None, Terrain::example(), Some(client))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_destruct_request_returns_error() {
        let session_id = std::env::var(TERRAIN_SESSION_ID).ok();

        let terrain_dir = PathBuf::from(TEST_TERRAIN_DIR);
        let toml_path = terrain_dir.join(TERRAIN_TOML);

        let mut context = Context::build(
            terrain_dir,
            PathBuf::new(),
            toml_path,
            Config::default(),
            MockExecutor::new(),
        );

        if let Some(session_id) = &session_id {
            context = context.set_session_id(session_id.clone());
        }

        let expected_error = "failed to parse execute request".to_string();

        let client = ExpectClient::to_send(ProtoRequest::Execute(
            expected_request_destruct_example_biome(session_id),
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
