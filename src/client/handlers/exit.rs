use crate::client::args::BiomeArg;
use crate::client::handlers::background;
#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::environment::Environment;
use crate::client::types::terrain::Terrain;
use crate::common::constants::{DESTRUCTORS, TERRAIN_AUTO_APPLY, TERRAIN_SELECTED_BIOME};
use anyhow::{bail, Context as AnyhowContext, Result};
use std::collections::BTreeMap;
use std::env;
use std::str::FromStr;

pub async fn handle(context: Context, terrain: Terrain, client: Option<Client>) -> Result<()> {
    let session_id = context.session_id();
    let selected_biome = env::var(TERRAIN_SELECTED_BIOME).unwrap_or_default();

    if session_id.is_empty() || selected_biome.is_empty() {
        bail!("no active terrain found, use 'terrainium enter' command to activate a terrain.");
    }

    if should_run_destructor() {
        let environment = Environment::from(
            &terrain,
            BiomeArg::from_str(&selected_biome).unwrap(),
            context.terrain_dir(),
        )
        .context("failed to generate environment")?;

        background::handle(
            &context,
            DESTRUCTORS,
            environment,
            Some(BTreeMap::<String, String>::new()),
            client,
        )
        .await
        .context("failed to run destructors")?;
    }

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

#[cfg(test)]
mod tests {}
