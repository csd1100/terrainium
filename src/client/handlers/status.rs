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
        Client::new(PathBuf::from(TERRAINIUMD_SOCKET)).await?
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
