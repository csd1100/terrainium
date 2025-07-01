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
