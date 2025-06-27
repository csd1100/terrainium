use anyhow::Result;

use crate::client::args::BiomeArg;
use crate::client::handlers::background;
#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use crate::common::utils::timestamp;

pub async fn handle(
    context: Context,
    biome: BiomeArg,
    terrain: Terrain,
    client: Option<Client>,
) -> Result<()> {
    background::handle(context, biome, terrain, true, timestamp(), client).await
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::client::args::BiomeArg;
    use crate::client::test_utils::assertions::client::ExpectClient;
    use crate::client::types::client::MockClient;
    use crate::client::types::context::Context;
    use crate::client::types::proto::ProtoRequest;
    use crate::client::types::terrain::Terrain;
    use crate::common::constants::TERRAIN_SESSION_ID;
    use crate::common::execute::MockExecutor;
    use crate::common::test_utils::{TEST_TERRAIN_DIR, expected_execute_request_example_biome};
    use crate::common::types::pb;

    pub(crate) fn expected_construct_request_example_biome(
        session_id: Option<String>,
    ) -> pb::Execute {
        expected_execute_request_example_biome(session_id, true)
    }

    #[tokio::test]
    async fn test_construct_sends_request() {
        let session_id = std::env::var(TERRAIN_SESSION_ID).ok();

        let mut context = Context::build(
            Path::new(TEST_TERRAIN_DIR),
            Path::new(""),
            false,
            MockExecutor::new(),
        );

        if let Some(session_id) = &session_id {
            context = context.set_session_id(session_id);
        }

        let client = ExpectClient::send(ProtoRequest::Execute(
            expected_construct_request_example_biome(session_id),
        ))
        .successfully();

        super::handle(context, BiomeArg::Default, Terrain::example(), Some(client))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_construct_does_not_send_request_if_no_background() {
        let context = Context::build(
            Path::new(TEST_TERRAIN_DIR),
            Path::new(""),
            false,
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

        let mut context = Context::build(
            Path::new(TEST_TERRAIN_DIR),
            Path::new(""),
            false,
            MockExecutor::new(),
        );

        if let Some(session_id) = &session_id {
            context = context.set_session_id(session_id);
        }

        let expected_error = "failed to parse execute request";

        let client = ExpectClient::send(ProtoRequest::Execute(
            expected_construct_request_example_biome(session_id),
        ))
        .with_returning_error(expected_error);

        let actual_error =
            super::handle(context, BiomeArg::Default, Terrain::example(), Some(client))
                .await
                .expect_err("expected error")
                .to_string();

        assert_eq!(actual_error, expected_error);
    }
}
