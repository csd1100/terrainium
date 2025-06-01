use crate::client::args::BiomeArg;
use crate::client::handlers::background;
#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::environment::Environment;
use crate::client::types::terrain::Terrain;
use crate::common::constants::DESTRUCTORS;
use anyhow::{Context as AnyhowContext, Result};

pub async fn handle(
    context: Context,
    biome: BiomeArg,
    terrain: Terrain,
    client: Option<Client>,
) -> Result<()> {
    let environment = Environment::from(&terrain, biome, context.terrain_dir())
        .context("failed to generate environment")?;
    background::handle(&context, DESTRUCTORS, environment, None, client).await
}

#[cfg(test)]
mod tests {}
