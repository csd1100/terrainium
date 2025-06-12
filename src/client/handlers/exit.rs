use crate::client::args::BiomeArg;
use crate::client::handlers::background::execute_request;
#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::environment::Environment;
use crate::client::types::proto::ProtoRequest;
use crate::client::types::terrain::Terrain;
use crate::common::constants::{TERRAINIUMD_SOCKET, TERRAIN_AUTO_APPLY, TERRAIN_SELECTED_BIOME};
use crate::common::types::pb;
use crate::common::utils::timestamp;
use anyhow::{bail, Context as AnyhowContext, Result};
use std::env;
use std::path::PathBuf;
use std::str::FromStr;

pub async fn handle(context: Context, terrain: Terrain, client: Option<Client>) -> Result<()> {
    let session_id = context.session_id();
    let selected_biome = env::var(TERRAIN_SELECTED_BIOME).unwrap_or_default();

    if session_id.is_none() || selected_biome.is_empty() {
        bail!("no active terrain found, use 'terrainium enter' command to activate a terrain.");
    }

    let mut client = if let Some(client) = client {
        client
    } else {
        Client::new(PathBuf::from(TERRAINIUMD_SOCKET)).await?
    };

    client
        .request(ProtoRequest::Deactivate(deactivate(
            terrain.name().to_string(),
            session_id.expect("session id to be present").to_string(),
            selected_biome,
            terrain,
            context,
        )?))
        .await?;
    Ok(())
}

/// 'terrainium exit' should run background destructor commands only in following case:
/// 1. Auto-apply is disabled
/// 2. Auto-apply is enabled but background flag is also turned on
fn should_run_destructor() -> bool {
    let auto_apply = env::var(TERRAIN_AUTO_APPLY);
    match auto_apply {
        Ok(auto_apply) => auto_apply == "all" || auto_apply == "background",
        Err(_) => true,
    }
}

fn deactivate(
    terrain_name: String,
    session_id: String,
    selected_biome: String,
    terrain: Terrain,
    context: Context,
) -> Result<pb::Deactivate> {
    let end_timestamp = timestamp();
    let destructors = if should_run_destructor() {
        let environment = Environment::from(
            &terrain,
            BiomeArg::from_str(&selected_biome).unwrap(),
            context.terrain_dir(),
        )
        .context("failed to generate environment")?;
        execute_request(&context, environment, false, end_timestamp.clone())?
    } else {
        None
    };

    Ok(pb::Deactivate {
        session_id,
        terrain_name,
        end_timestamp,
        destructors,
    })
}

#[cfg(test)]
mod tests {}
