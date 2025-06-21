#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::proto::{ProtoRequest, ProtoResponse};
use crate::client::types::terrain::Terrain;
use crate::common::constants::get_terrainiumd_socket;
use crate::common::types::pb;
use crate::common::types::terrain_state::TerrainState;
use anyhow::{bail, Context as AnyhowContext, Result};
use std::path::PathBuf;

pub async fn handle(
    context: Context,
    terrain: Terrain,
    json: bool,
    session_id: Option<String>,
    recent: Option<u32>,
    client: Option<Client>,
) -> Result<()> {
    let mut client = if let Some(client) = client {
        client
    } else {
        Client::new(PathBuf::from(get_terrainiumd_socket())).await?
    };

    let response = client
        .request(ProtoRequest::Status(status(
            context, session_id, recent, terrain,
        )))
        .await?;

    if let ProtoResponse::Status(status) = response {
        let status: TerrainState = status.try_into().context("failed to convert status")?;
        let status = if json {
            serde_json::to_string_pretty(&status).context("failed to serialize status")?
        } else {
            format!("{}", status)
        };
        println!("{status}");
    } else {
        bail!("invalid status response from daemon");
    }

    Ok(())
}

fn status(
    context: Context,
    session_id: Option<String>,
    recent: Option<u32>,
    terrain: Terrain,
) -> pb::StatusRequest {
    let identifier = match session_id {
        Some(session_id) => pb::status_request::Identifier::SessionId(session_id),
        None => match recent {
            None => {
                if let Some(session_id) = context.session_id() {
                    pb::status_request::Identifier::SessionId(session_id)
                } else {
                    pb::status_request::Identifier::Recent(0)
                }
            }
            Some(recent) => pb::status_request::Identifier::Recent(recent),
        },
    };

    pb::StatusRequest {
        terrain_name: terrain.name().to_string(),
        identifier: Some(identifier),
    }
}

#[cfg(test)]
mod tests {
    use crate::client::test_utils::assertions::client::ExpectClient;
    use crate::client::test_utils::expected_env_vars_example_biome;
    use crate::client::types::context::Context;
    use crate::client::types::proto::{ProtoRequest, ProtoResponse};
    use crate::client::types::terrain::Terrain;
    use crate::common::constants::{EXAMPLE_BIOME, TERRAIN_TOML, TEST_TIMESTAMP};
    use crate::common::execute::MockExecutor;
    use crate::common::test_utils;
    use crate::common::test_utils::{RequestFor, TEST_SESSION_ID, TEST_TERRAIN_NAME};
    use crate::common::types::pb;
    use std::path::Path;
    use tempfile::tempdir;

    fn expected_status_response(session_id: &str, terrain_dir: &Path) -> pb::StatusResponse {
        pb::StatusResponse {
            session_id: session_id.to_string(),
            terrain_name: TEST_TERRAIN_NAME.to_string(),
            biome_name: EXAMPLE_BIOME.to_string(),
            terrain_dir: terrain_dir.to_string_lossy().to_string(),
            toml_path: terrain_dir.join(TERRAIN_TOML).to_string_lossy().to_string(),
            is_background: false,
            start_timestamp: TEST_TIMESTAMP.to_string(),
            end_timestamp: TEST_TIMESTAMP.to_string(),
            envs: expected_env_vars_example_biome(terrain_dir),
            constructors: Default::default(),
            destructors: Default::default(),
        }
    }

    #[tokio::test]
    async fn returns_status_for_current() {
        let session_id = TEST_SESSION_ID;
        let terrain_dir = tempdir().unwrap();

        let context = Context::build(
            terrain_dir.path(),
            Path::new(""),
            false,
            MockExecutor::default(),
        )
        .set_session_id(session_id);

        let client = ExpectClient::send(ProtoRequest::Status(test_utils::expected_status_request(
            RequestFor::None,
            session_id,
        )))
        .with_expected_response(ProtoResponse::Status(Box::from(expected_status_response(
            session_id,
            terrain_dir.path(),
        ))))
        .successfully();

        super::handle(context, Terrain::example(), false, None, None, Some(client))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn returns_status_for_specified_session_id() {
        let session_id = "some-session-id";
        let terrain_dir = tempdir().unwrap();

        let context = Context::build(
            terrain_dir.path(),
            Path::new(""),
            false,
            MockExecutor::default(),
        )
        .set_session_id(session_id);

        let client = ExpectClient::send(ProtoRequest::Status(test_utils::expected_status_request(
            RequestFor::SessionId(session_id.to_string()),
            "",
        )))
        .with_expected_response(ProtoResponse::Status(Box::from(expected_status_response(
            session_id,
            terrain_dir.path(),
        ))))
        .successfully();

        super::handle(
            context,
            Terrain::example(),
            false,
            Some(session_id.to_string()),
            None,
            Some(client),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn returns_status_for_specified_recent() {
        let terrain_dir = tempdir().unwrap();

        let context = Context::build(
            terrain_dir.path(),
            Path::new(""),
            false,
            MockExecutor::default(),
        );

        let client = ExpectClient::send(ProtoRequest::Status(test_utils::expected_status_request(
            RequestFor::Recent(1),
            "",
        )))
        .with_expected_response(ProtoResponse::Status(Box::from(expected_status_response(
            TEST_SESSION_ID,
            terrain_dir.path(),
        ))))
        .successfully();

        super::handle(
            context,
            Terrain::example(),
            false,
            None,
            Some(1),
            Some(client),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn returns_status_for_no_recent_no_session_id() {
        let terrain_dir = tempdir().unwrap();

        let context = Context::build(
            terrain_dir.path(),
            Path::new(""),
            false,
            MockExecutor::default(),
        );

        let client = ExpectClient::send(ProtoRequest::Status(test_utils::expected_status_request(
            RequestFor::None,
            "",
        )))
        .with_expected_response(ProtoResponse::Status(Box::from(expected_status_response(
            TEST_SESSION_ID,
            terrain_dir.path(),
        ))))
        .successfully();

        super::handle(context, Terrain::example(), false, None, None, Some(client))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn returns_no_error_for_json() {
        let terrain_dir = tempdir().unwrap();

        let context = Context::build(
            terrain_dir.path(),
            Path::new(""),
            false,
            MockExecutor::default(),
        )
        .set_session_id(TEST_SESSION_ID);

        let client = ExpectClient::send(ProtoRequest::Status(test_utils::expected_status_request(
            RequestFor::None,
            TEST_SESSION_ID,
        )))
        .with_expected_response(ProtoResponse::Status(Box::from(expected_status_response(
            TEST_SESSION_ID,
            terrain_dir.path(),
        ))))
        .successfully();

        super::handle(context, Terrain::example(), true, None, None, Some(client))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn returns_error_for_invalid_response() {
        let terrain_dir = tempdir().unwrap();

        let context = Context::build(
            terrain_dir.path(),
            Path::new(""),
            false,
            MockExecutor::default(),
        )
        .set_session_id(TEST_SESSION_ID);

        let client = ExpectClient::send(ProtoRequest::Status(test_utils::expected_status_request(
            RequestFor::None,
            TEST_SESSION_ID,
        )))
        .with_expected_response(ProtoResponse::Success)
        .successfully();

        let error = super::handle(context, Terrain::example(), true, None, None, Some(client))
            .await
            .unwrap_err()
            .to_string();

        assert_eq!(error, "invalid status response from daemon");
    }
}
