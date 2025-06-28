use anyhow::{Context as AnyhowContext, Result, bail};

#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::proto::{ProtoRequest, ProtoResponse};
use crate::common::constants::TERRAIN_SESSION_ID;
use crate::common::types::paths::get_terrainiumd_paths;
use crate::common::types::pb;
use crate::common::types::terrain_state::TerrainState;

pub async fn handle(
    json: bool,
    terrain_name: String,
    session_id: Option<String>,
    recent: Option<u32>,
    client: Option<Client>,
) -> Result<()> {
    let mut client = if let Some(client) = client {
        client
    } else {
        Client::new(get_terrainiumd_paths().socket()).await?
    };

    let response = client
        .request(ProtoRequest::Status(status(
            terrain_name,
            session_id,
            recent,
        )))
        .await?;

    if let ProtoResponse::Status(status) = response {
        let status: TerrainState = status.try_into().context("failed to convert status")?;
        let status = if json {
            serde_json::to_string_pretty(&status).context("failed to serialize status")?
        } else {
            format!("{status}")
        };
        println!("{status}");
    } else {
        bail!("invalid status response from daemon");
    }

    Ok(())
}

fn status(
    terrain_name: String,
    session_id: Option<String>,
    recent: Option<u32>,
) -> pb::StatusRequest {
    let identifier = match session_id {
        Some(session_id) => pb::status_request::Identifier::SessionId(session_id),
        None => match recent {
            None => {
                if let Ok(session_id) = std::env::var(TERRAIN_SESSION_ID) {
                    pb::status_request::Identifier::SessionId(session_id)
                } else {
                    pb::status_request::Identifier::Recent(0)
                }
            }

            Some(recent) => pb::status_request::Identifier::Recent(recent),
        },
    };

    pb::StatusRequest {
        terrain_name,
        identifier: Some(identifier),
    }
}

#[cfg(test)]
mod tests {
    use std::env::VarError;
    use std::path::Path;

    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    use crate::client::test_utils::assertions::client::ExpectClient;
    use crate::client::test_utils::{restore_env_var, set_env_var};
    use crate::client::types::proto::{ProtoRequest, ProtoResponse};
    use crate::common::constants::{
        EXAMPLE_BIOME, TERRAIN_SESSION_ID, TERRAIN_TOML, TEST_TIMESTAMP,
    };
    use crate::common::test_utils;
    use crate::common::test_utils::{
        RequestFor, TEST_SESSION_ID, TEST_TERRAIN_NAME, expected_env_vars_example_biome,
    };
    use crate::common::types::pb;

    fn expected_status_response(
        terrain_name: &str,
        session_id: &str,
        terrain_dir: &Path,
    ) -> pb::StatusResponse {
        pb::StatusResponse {
            session_id: session_id.to_string(),
            terrain_name: terrain_name.to_string(),
            biome_name: EXAMPLE_BIOME.to_string(),
            terrain_dir: terrain_dir.to_string_lossy().to_string(),
            toml_path: terrain_dir.join(TERRAIN_TOML).to_string_lossy().to_string(),
            is_background: false,
            start_timestamp: TEST_TIMESTAMP.to_string(),
            end_timestamp: TEST_TIMESTAMP.to_string(),
            envs: expected_env_vars_example_biome(),
            constructors: Default::default(),
            destructors: Default::default(),
        }
    }

    #[tokio::test]
    async fn returns_status_for_specified_session_id() {
        let session_id = "some-session-id";
        let terrain_dir = tempdir().unwrap();

        let client = ExpectClient::send(ProtoRequest::Status(test_utils::expected_status_request(
            RequestFor::SessionId(session_id.to_string()),
            "",
        )))
        .with_expected_response(ProtoResponse::Status(Box::from(expected_status_response(
            TEST_TERRAIN_NAME,
            session_id,
            terrain_dir.path(),
        ))))
        .successfully();

        super::handle(
            false,
            TEST_TERRAIN_NAME.to_string(),
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

        let client = ExpectClient::send(ProtoRequest::Status(test_utils::expected_status_request(
            RequestFor::Recent(1),
            "",
        )))
        .with_expected_response(ProtoResponse::Status(Box::from(expected_status_response(
            TEST_TERRAIN_NAME,
            TEST_SESSION_ID,
            terrain_dir.path(),
        ))))
        .successfully();

        super::handle(
            false,
            TEST_TERRAIN_NAME.to_string(),
            None,
            Some(1),
            Some(client),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn returns_status_for_no_recent_no_session_id() {
        let session_id: std::result::Result<String, VarError>;
        unsafe {
            session_id = set_env_var(TERRAIN_SESSION_ID, Some(TEST_SESSION_ID));
        }
        let terrain_dir = tempdir().unwrap();

        let client = ExpectClient::send(ProtoRequest::Status(test_utils::expected_status_request(
            RequestFor::SessionId(TEST_SESSION_ID.to_string()),
            "",
        )))
        .with_expected_response(ProtoResponse::Status(Box::from(expected_status_response(
            TEST_TERRAIN_NAME,
            TEST_SESSION_ID,
            terrain_dir.path(),
        ))))
        .successfully();

        super::handle(
            false,
            TEST_TERRAIN_NAME.to_string(),
            None,
            None,
            Some(client),
        )
        .await
        .unwrap();
        unsafe {
            restore_env_var(TERRAIN_SESSION_ID, session_id);
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn returns_status_for_no_recent_no_session_id_no_env() {
        let session_id: std::result::Result<String, VarError>;
        unsafe {
            session_id = set_env_var(TERRAIN_SESSION_ID, None);
        }
        let terrain_dir = tempdir().unwrap();

        let client = ExpectClient::send(ProtoRequest::Status(test_utils::expected_status_request(
            RequestFor::Recent(0),
            "",
        )))
        .with_expected_response(ProtoResponse::Status(Box::from(expected_status_response(
            TEST_TERRAIN_NAME,
            TEST_SESSION_ID,
            terrain_dir.path(),
        ))))
        .successfully();

        super::handle(
            false,
            TEST_TERRAIN_NAME.to_string(),
            None,
            None,
            Some(client),
        )
        .await
        .unwrap();
        unsafe {
            restore_env_var(TERRAIN_SESSION_ID, session_id);
        }
    }

    #[tokio::test]
    async fn returns_no_error_for_json() {
        let terrain_dir = tempdir().unwrap();

        let client = ExpectClient::send(ProtoRequest::Status(test_utils::expected_status_request(
            RequestFor::None,
            TEST_SESSION_ID,
        )))
        .with_expected_response(ProtoResponse::Status(Box::from(expected_status_response(
            TEST_TERRAIN_NAME,
            TEST_SESSION_ID,
            terrain_dir.path(),
        ))))
        .successfully();

        super::handle(
            true,
            TEST_TERRAIN_NAME.to_string(),
            Some(TEST_SESSION_ID.to_string()),
            None,
            Some(client),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn returns_error_for_invalid_response() {
        let client = ExpectClient::send(ProtoRequest::Status(test_utils::expected_status_request(
            RequestFor::None,
            TEST_SESSION_ID,
        )))
        .with_expected_response(ProtoResponse::Success)
        .successfully();

        let error = super::handle(
            true,
            TEST_TERRAIN_NAME.to_string(),
            Some(TEST_SESSION_ID.to_string()),
            None,
            Some(client),
        )
        .await
        .unwrap_err()
        .to_string();

        assert_eq!(error, "invalid status response from daemon");
    }
}
