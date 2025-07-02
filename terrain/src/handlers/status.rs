use anyhow::{Context, Result, bail};
use terrainium_lib::paths::DaemonPaths;
use terrainium_lib::pb;
use terrainium_lib::socket::Socket;
use terrainium_lib::state::TerrainState;

use crate::constants::TERRAIN_SESSION_ID;
use crate::handlers::Client;

/// Returns status for specified terrain and session
pub async fn handle(
    json: bool,
    terrain_name: String,
    session_id: Option<String>,
    recent: Option<u32>,
    client: Option<Box<dyn Socket<pb::Response, pb::Request> + Send>>,
) -> Result<()> {
    let mut client = if let Some(client) = client {
        client
    } else {
        Box::new(Client::new(DaemonPaths::get().socket()).context("failed to create the client")?)
    };

    let body = status(terrain_name, session_id, recent);
    let payload = pb::Request {
        r#type: pb::request::RequestType::Stauts as i32,
        payload: Some(pb::request::Payload::Status(body)),
    };

    let response = client
        .request(&payload)
        .await
        .context("failed to read response from daemon")?;

    if response.payload.is_none() {
        bail!("invalid empty response from daemon");
    }

    let status = match response.payload.unwrap() {
        pb::response::Payload::Error(err) => bail!("error received from daemon: {err}"),
        pb::response::Payload::Body(pb::Body { message: None }) => {
            bail!("expected status from the daemon but none found")
        }
        pb::response::Payload::Body(pb::Body { message }) => message.unwrap(),
    };

    let status: TerrainState = status.try_into().context("failed to convert status")?;
    let status = if json {
        serde_json::to_string_pretty(&status).context("failed to serialize status")?
    } else {
        format!("{status}")
    };

    println!("{status}");

    Ok(())
}

/// Constructs [pb::StatusRequest]
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
    use terrainium_lib::pb;
    use terrainium_lib::test_utils::mocket::Mocket;
    use terrainium_lib::test_utils::{restore_env_var, set_env_var};

    use crate::constants::{
        EXAMPLE_BIOME, TERRAIN_SESSION_ID, TERRAIN_TOML, TERRAINIUM, TEST_TIMESTAMP,
    };
    use crate::test_helpers::{TEST_SESSION_ID, expected_env_vars_example_biome};

    pub enum RequestFor {
        SessionId(String),
        Recent(u32),
        None,
    }

    pub fn expected_status_request(
        request_for: RequestFor,
        current_session_id: &str,
    ) -> pb::StatusRequest {
        pb::StatusRequest {
            terrain_name: TERRAINIUM.to_string(),
            identifier: {
                let id = match request_for {
                    RequestFor::SessionId(session_id) => {
                        pb::status_request::Identifier::SessionId(session_id)
                    }
                    RequestFor::Recent(r) => pb::status_request::Identifier::Recent(r),
                    RequestFor::None => {
                        if current_session_id.is_empty() {
                            pb::status_request::Identifier::Recent(0)
                        } else {
                            pb::status_request::Identifier::SessionId(
                                current_session_id.to_string(),
                            )
                        }
                    }
                };
                Some(id)
            },
        }
    }

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
        let terrain_dir = tempdir().unwrap();
        let session_id = "some-session-id";

        let req = pb::Request {
            r#type: pb::request::RequestType::Stauts as i32,
            payload: Some(pb::request::Payload::Status(expected_status_request(
                RequestFor::SessionId(session_id.to_string()),
                "",
            ))),
        };
        let res = pb::Response {
            payload: Some(pb::response::Payload::Body(pb::Body {
                message: Some(expected_status_response(
                    TERRAINIUM,
                    session_id,
                    terrain_dir.path(),
                )),
            })),
        };

        let client = Mocket::<pb::Response, pb::Request>::to()
            .send(req)
            .receive(res)
            .successfully();

        super::handle(
            false,
            TERRAINIUM.to_string(),
            Some(session_id.to_string()),
            None,
            Some(Box::new(client)),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn returns_status_for_specified_recent() {
        let terrain_dir = tempdir().unwrap();

        let req = pb::Request {
            r#type: pb::request::RequestType::Stauts as i32,
            payload: Some(pb::request::Payload::Status(expected_status_request(
                RequestFor::Recent(1),
                "",
            ))),
        };
        let res = pb::Response {
            payload: Some(pb::response::Payload::Body(pb::Body {
                message: Some(expected_status_response(
                    TERRAINIUM,
                    TEST_SESSION_ID,
                    terrain_dir.path(),
                )),
            })),
        };

        let client = Mocket::<pb::Response, pb::Request>::to()
            .send(req)
            .receive(res)
            .successfully();

        super::handle(
            false,
            TERRAINIUM.to_string(),
            None,
            Some(1),
            Some(Box::new(client)),
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

        let req = pb::Request {
            r#type: pb::request::RequestType::Stauts as i32,
            payload: Some(pb::request::Payload::Status(expected_status_request(
                RequestFor::SessionId(TEST_SESSION_ID.to_string()),
                "",
            ))),
        };
        let res = pb::Response {
            payload: Some(pb::response::Payload::Body(pb::Body {
                message: Some(expected_status_response(
                    TERRAINIUM,
                    TEST_SESSION_ID,
                    terrain_dir.path(),
                )),
            })),
        };

        let client = Mocket::<pb::Response, pb::Request>::to()
            .send(req)
            .receive(res)
            .successfully();

        super::handle(
            false,
            TERRAINIUM.to_string(),
            None,
            None,
            Some(Box::new(client)),
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

        let req = pb::Request {
            r#type: pb::request::RequestType::Stauts as i32,
            payload: Some(pb::request::Payload::Status(expected_status_request(
                RequestFor::Recent(0),
                "",
            ))),
        };
        let res = pb::Response {
            payload: Some(pb::response::Payload::Body(pb::Body {
                message: Some(expected_status_response(
                    TERRAINIUM,
                    TEST_SESSION_ID,
                    terrain_dir.path(),
                )),
            })),
        };

        let client = Mocket::<pb::Response, pb::Request>::to()
            .send(req)
            .receive(res)
            .successfully();

        super::handle(
            false,
            TERRAINIUM.to_string(),
            None,
            None,
            Some(Box::new(client)),
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

        let req = pb::Request {
            r#type: pb::request::RequestType::Stauts as i32,
            payload: Some(pb::request::Payload::Status(expected_status_request(
                RequestFor::None,
                TEST_SESSION_ID,
            ))),
        };
        let res = pb::Response {
            payload: Some(pb::response::Payload::Body(pb::Body {
                message: Some(expected_status_response(
                    TERRAINIUM,
                    TEST_SESSION_ID,
                    terrain_dir.path(),
                )),
            })),
        };

        let client = Mocket::<pb::Response, pb::Request>::to()
            .send(req)
            .receive(res)
            .successfully();

        super::handle(
            true,
            TERRAINIUM.to_string(),
            Some(TEST_SESSION_ID.to_string()),
            None,
            Some(Box::new(client)),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn returns_error_for_invalid_response() {
        let req = pb::Request {
            r#type: pb::request::RequestType::Stauts as i32,
            payload: Some(pb::request::Payload::Status(expected_status_request(
                RequestFor::None,
                TEST_SESSION_ID,
            ))),
        };
        let res = pb::Response {
            payload: Some(pb::response::Payload::Body(pb::Body { message: None })),
        };

        let client = Mocket::<pb::Response, pb::Request>::to()
            .send(req)
            .receive(res)
            .successfully();

        let error = super::handle(
            true,
            TERRAINIUM.to_string(),
            Some(TEST_SESSION_ID.to_string()),
            None,
            Some(Box::new(client)),
        )
        .await
        .unwrap_err()
        .to_string();

        assert_eq!(error, "expected status from the daemon but none found");
    }
}
