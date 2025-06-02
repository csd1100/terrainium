use crate::client::args::BiomeArg;
#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::environment::Environment;
use crate::client::types::proto::ProtoRequest;
use crate::client::types::terrain::Terrain;
use crate::common::constants::{TERRAINIUMD_SOCKET, TERRAIN_SESSION_ID};
use crate::common::types::pb;
use crate::common::types::pb::Construct;
use crate::common::utils::timestamp;
use anyhow::{Context as AnyhowContext, Result};
use std::path::PathBuf;

pub async fn handle(
    context: Context,
    biome: BiomeArg,
    terrain: Terrain,
    client: Option<Client>,
) -> Result<()> {
    let environment = Environment::from(&terrain, biome, context.terrain_dir())
        .context("failed to generate environment")?;

    let mut client: Client = if let Some(client) = client {
        client
    } else {
        Client::new(PathBuf::from(TERRAINIUMD_SOCKET)).await?
    };

    client
        .request(ProtoRequest::Construct(construct(context, environment)?))
        .await?;

    Ok(())
}

fn construct(context: Context, environment: Environment) -> Result<Construct> {
    let commands: Vec<pb::Command> = environment
        .constructors()
        .to_proto_commands(environment.envs())
        .context("failed to convert commands")?;

    Ok(Construct {
        session_id: std::env::var(TERRAIN_SESSION_ID).ok(),
        terrain_name: environment.name().to_string(),
        biome_name: environment.selected_biome().to_string(),
        toml_path: context.toml_path().to_string_lossy().to_string(),
        timestamp: timestamp(),
        commands,
    })
}

#[cfg(test)]
mod tests {}
