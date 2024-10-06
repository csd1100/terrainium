use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::common::constants::TERRAINIUM_ENABLED;
use crate::common::types::pb;
use crate::common::types::pb::{Error, StatusPoll};
use crate::common::types::socket::Socket;
use crate::common::types::terrain_state::TerrainState;
use anyhow::{anyhow, Context as AnyhowContext, Result};
use prost_types::Any;
use std::env;

pub async fn handle(context: Context, json: bool, mut client: Client) -> Result<()> {
    let is_terrain_enabled = env::var(TERRAINIUM_ENABLED).unwrap_or_else(|_| "false".to_string());

    if is_terrain_enabled != "true" {
        return Err(anyhow!(
            "no active terrain found, use `terrainium enter` command to activate a terrain."
        ));
    }

    let session_id = context.session_id();
    let terrain_name = context.name();

    let request = pb::StatusRequest {
        session_id: session_id.clone(),
        terrain_name: terrain_name.clone(),
    };

    client
        .write_and_stop(Any::from_msg(&request).unwrap())
        .await?;

    let response: Any = client.read().await?;

    let status_response: Result<pb::StatusResponse> = response
        .to_msg()
        .context("failed to convert status response from any");

    if let Ok(status) = status_response {
        let last_modified = status.last_modified.clone();
        let status: TerrainState = status.into();
        let poll_request: StatusPoll = StatusPoll {
            session_id,
            terrain_name,
            last_modified,
        };

        status
            .render(json, poll_request)
            .context("status to be rendered")?;
    } else {
        let error: Error = response
            .to_msg()
            .context("failed to convert to error from Any")?;

        return Err(anyhow!(
            "error response from daemon {}",
            error.error_message
        ));
    }
    Ok(())
}
