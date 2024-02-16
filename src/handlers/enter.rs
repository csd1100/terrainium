use std::collections::HashMap;

use anyhow::{Context, Result};
use mockall_double::double;
use uuid::Uuid;

use crate::{
    helpers::constants::{TERRAINIUM_ENABLED, TERRAINIUM_SESSION_ID},
    helpers::helpers::merge_hashmaps,
    shell::zsh::{get_zsh_envs, spawn_zsh},
    types::args::BiomeArg,
};

use super::construct;

#[double]
use crate::helpers::helpers::fs;

pub fn handle(biome: Option<BiomeArg>) -> Result<()> {
    let terrain = fs::get_parsed_terrain()?;
    let selected = terrain
        .get(biome.clone())
        .context("unable to select biome")?;
    let mut envs = selected.env;

    if envs.is_none() {
        envs = Some(HashMap::<String, String>::new());
    }

    if let Some(envs) = envs.as_mut() {
        envs.insert(TERRAINIUM_ENABLED.to_string(), "1".to_string());
        envs.insert(
            TERRAINIUM_SESSION_ID.to_string(),
            Uuid::new_v4().to_string(),
        );
    }

    if let Some(envs) = envs {
        let zsh_env = get_zsh_envs(terrain.get_selected_biome_name(&biome)?)
            .context("unable to set zsh environment varibles")?;
        let merged = merge_hashmaps(&envs.clone(), &zsh_env);

        construct::handle(biome, Some(&merged)).context("unable to construct biome")?;
        spawn_zsh(vec!["-s"], Some(merged)).context("unable to start zsh")?;
    }

    Ok(())
}
