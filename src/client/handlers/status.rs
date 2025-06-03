#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::proto::{ProtoRequest, ProtoResponse};
use crate::client::types::terrain::Terrain;
use crate::common::constants::TERRAINIUMD_SOCKET;
use crate::common::types::pb;
use crate::common::types::terrain_state::TerrainState;
use anyhow::{bail, Context as AnyhowContext, Result};
use std::path::PathBuf;

pub async fn handle(context: Context, terrain: Terrain, client: Option<Client>) -> Result<()> {
    let mut client = if let Some(client) = client {
        client
    } else {
        Client::new(PathBuf::from(TERRAINIUMD_SOCKET)).await?
    };

    let response = client
        .request(ProtoRequest::Status(status(context, terrain)))
        .await?;

    if let ProtoResponse::Status(status) = response {
        let status: TerrainState = status.try_into().context("failed to convert status")?;
        let status = serde_json::to_string_pretty(&status).context("failed to serialize status")?;
        println!("{status}");
    } else {
        bail!("invalid status response from daemon");
    }

    Ok(())
}

fn status(context: Context, terrain: Terrain) -> pb::StatusRequest {
    pb::StatusRequest {
        session_id: context.session_id().to_string(),
        terrain_name: terrain.name().to_string(),
    }
}
