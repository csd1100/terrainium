use crate::client::args::BiomeArg;
#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::environment::Environment;
use crate::client::types::proto::ProtoRequest;
use crate::client::types::terrain::Terrain;
use crate::common::constants::TERRAINIUMD_SOCKET;
use crate::common::types::pb;
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
        .request(ProtoRequest::Execute(destruct(
            context,
            environment,
            timestamp(),
        )?))
        .await?;

    Ok(())
}

pub(crate) fn destruct(
    context: Context,
    environment: Environment,
    timestamp: String,
) -> Result<pb::Execute> {
    let commands: Vec<pb::Command> = environment
        .destructors()
        .to_proto_commands(environment.envs())
        .context("failed to convert commands")?;

    Ok(pb::Execute {
        session_id: context.session_id(),
        terrain_name: environment.name().to_string(),
        biome_name: environment.selected_biome().to_string(),
        terrain_dir: context.terrain_dir().to_string_lossy().to_string(),
        toml_path: context.toml_path().to_string_lossy().to_string(),
        is_constructor: false,
        timestamp,
        commands,
    })
}

#[cfg(test)]
mod tests {}
