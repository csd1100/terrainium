use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::common::constants::{TERRAINIUMD_SOCKET, TERRAIN_ENABLED};
use crate::common::types::pb;
use crate::common::types::pb::Error;
use crate::common::types::socket::Socket;
use crate::common::types::terrain_state::TerrainState;
use anyhow::{anyhow, Context as AnyhowContext, Result};
use prost_types::Any;
use std::env;
use std::path::PathBuf;

pub async fn handle(context: Context, json: bool, client: Option<Client>) -> Result<()> {
    let is_terrain_enabled = env::var(TERRAIN_ENABLED).unwrap_or_else(|_| "false".to_string());

    if is_terrain_enabled != "true" {
        return Err(anyhow!(
            "no active terrain found, use `terrainium enter` command to activate a terrain."
        ));
    }

    let mut client = if let Some(client) = client {
        client
    } else {
        Client::new(PathBuf::from(TERRAINIUMD_SOCKET)).await?
    };

    let session_id = context.session_id().to_string();
    let terrain_name = context.name();

    let request = pb::StatusRequest {
        session_id,
        terrain_name,
    };

    client
        .write_and_stop(Any::from_msg(&request).unwrap())
        .await?;

    let response: Any = client.read().await?;

    let status_response: Result<pb::StatusResponse> = response
        .to_msg()
        .context("failed to convert status response from any");

    if let Ok(status) = status_response {
        let status: TerrainState = status.into();
        status.render(json).context("status to be rendered")?;
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