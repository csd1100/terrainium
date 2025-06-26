#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::proto::{ProtoRequest, ProtoResponse};
use crate::common::constants::{TERRAIN_NAME, TERRAIN_SESSION_ID};
use crate::common::types::paths::get_terrainiumd_paths;
use crate::common::types::pb;
use crate::common::types::terrain_state::TerrainState;
use anyhow::{Context as AnyhowContext, Result, bail};

fn get_valid_terrain_name_session_id(
    terrain_name: Option<String>,
    session_id: Option<String>,
) -> Result<(String, Option<String>)> {
    let terrain_name_env = std::env::var(TERRAIN_NAME);
    let session_id_env = std::env::var(TERRAIN_SESSION_ID).ok();
    match terrain_name {
        None => Ok((
            terrain_name_env.context(
                "if there is no active terrain,terrain name should passed with `--terrain-name \
                 <NAME>`",
            )?,
            session_id.or(session_id_env),
        )),
        Some(terrain_name) => {
            if terrain_name_env.is_ok_and(|terrain_name_env| terrain_name_env != terrain_name) {
                // if terrain_name is specified and, it is different from env var
                // then do not use session id from environment variable as it will
                // be for different terrain
                Ok((terrain_name, session_id))
            } else {
                Ok((terrain_name, session_id.or(session_id_env)))
            }
        }
    }
}

pub async fn handle(
    json: bool,
    terrain_name: Option<String>,
    session_id: Option<String>,
    recent: Option<u32>,
    client: Option<Client>,
) -> Result<()> {
    let (terrain_name, session_id) = get_valid_terrain_name_session_id(terrain_name, session_id)?;

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
            format!("{}", status)
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
            None => pb::status_request::Identifier::Recent(0),
            Some(recent) => pb::status_request::Identifier::Recent(recent),
        },
    };

    pb::StatusRequest {
        terrain_name,
        identifier: Some(identifier),
    }
}

#[cfg(test)]
#[serial_test::serial]
mod tests {
    use crate::client::test_utils::assertions::client::ExpectClient;
    use crate::client::test_utils::{restore_env_var, set_env_var};
    use crate::client::types::proto::{ProtoRequest, ProtoResponse};
    use crate::common::constants::{
        EXAMPLE_BIOME, TERRAIN_NAME, TERRAIN_SESSION_ID, TERRAIN_TOML, TEST_TIMESTAMP,
    };
    use crate::common::test_utils;
    use crate::common::test_utils::expected_env_vars_example_biome;
    use crate::common::test_utils::{RequestFor, TEST_SESSION_ID, TEST_TERRAIN_NAME};
    use crate::common::types::pb;
    use std::env::VarError;
    use std::path::Path;
    use tempfile::tempdir;

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
            Some(TEST_TERRAIN_NAME.to_string()),
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
            Some(TEST_TERRAIN_NAME.to_string()),
            None,
            Some(1),
            Some(client),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn returns_error_no_terrain_name() {
        let error = super::handle(false, None, None, None, None)
            .await
            .expect_err("Should have returned error")
            .to_string();

        assert_eq!(
            error,
            "if there is no active terrain,terrain name should passed with `--terrain-name <NAME>`"
        );
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
            Some(TEST_TERRAIN_NAME.to_string()),
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
            Some(TEST_TERRAIN_NAME.to_string()),
            Some(TEST_SESSION_ID.to_string()),
            None,
            Some(client),
        )
        .await
        .unwrap_err()
        .to_string();

        assert_eq!(error, "invalid status response from daemon");
    }

    #[tokio::test]
    async fn returns_status_for_no_name_session_id() {
        let terrain_name: std::result::Result<String, VarError>;
        unsafe {
            terrain_name = set_env_var(TERRAIN_NAME, Some(TEST_TERRAIN_NAME));
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
            None,
            Some(TEST_SESSION_ID.to_string()),
            None,
            Some(client),
        )
        .await
        .unwrap();

        unsafe {
            restore_env_var(TERRAIN_NAME, terrain_name);
        }
    }

    #[tokio::test]
    async fn returns_status_for_same_name_no_session_id() {
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
            Some(TEST_TERRAIN_NAME.to_string()),
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
    async fn returns_status_for_no_name_no_session_id() {
        let terrain_name: std::result::Result<String, VarError>;
        let session_id: std::result::Result<String, VarError>;
        unsafe {
            terrain_name = set_env_var(TERRAIN_NAME, Some(TEST_TERRAIN_NAME));
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

        super::handle(false, None, None, None, Some(client))
            .await
            .unwrap();

        unsafe {
            restore_env_var(TERRAIN_SESSION_ID, session_id);
            restore_env_var(TERRAIN_NAME, terrain_name);
        }
    }

    #[tokio::test]
    async fn returns_status_recent_for_different_name_no_session_id() {
        let name = "some-terrain";
        let terrain_name: std::result::Result<String, VarError>;
        let session_id: std::result::Result<String, VarError>;
        unsafe {
            terrain_name = set_env_var(TERRAIN_NAME, Some(name));
            session_id = set_env_var(TERRAIN_SESSION_ID, Some("some-session-id"));
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
            Some(TEST_TERRAIN_NAME.to_string()),
            None,
            None,
            Some(client),
        )
        .await
        .unwrap();

        unsafe {
            restore_env_var(TERRAIN_SESSION_ID, session_id);
            restore_env_var(TERRAIN_NAME, terrain_name);
        }
    }

    #[tokio::test]
    async fn returns_status_recent_for_different_name_session_id() {
        let terrain_name: std::result::Result<String, VarError>;
        let session_id: std::result::Result<String, VarError>;
        unsafe {
            terrain_name = set_env_var(TERRAIN_NAME, Some("some-terrain"));
            session_id = set_env_var(TERRAIN_SESSION_ID, Some("some-session-id"));
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
            Some(TEST_TERRAIN_NAME.to_string()),
            Some(TEST_SESSION_ID.to_string()),
            None,
            Some(client),
        )
        .await
        .unwrap();

        unsafe {
            restore_env_var(TERRAIN_SESSION_ID, session_id);
            restore_env_var(TERRAIN_NAME, terrain_name);
        }
    }
}
