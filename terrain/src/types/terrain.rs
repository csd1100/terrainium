use std::collections::BTreeMap;
use std::fmt::Display;

use anyhow::{bail, Result};
use terrainium_lib::command::Command;

use crate::constants::{EDITOR, ENV_VAR, EXAMPLE_BIOME, NONE, TENTER, TERRAINIUM};
use crate::types::biome::Biome;
use crate::types::commands::Commands;
use crate::types::environment::Environment;

const AUTO_APPLY_ENABLED: &str = "enabled";
const AUTO_APPLY_BACKGROUND: &str = "background";
const AUTO_APPLY_REPLACE: &str = "replace";
const AUTO_APPLY_ALL: &str = "all";
const AUTO_APPLY_OFF: &str = "off";

#[derive(Default)]
pub enum AutoApply {
    All,
    Background,
    Replace,
    Enabled,
    #[default]
    Off,
}

impl Display for AutoApply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match &self {
            AutoApply::All => AUTO_APPLY_ALL,
            AutoApply::Background => AUTO_APPLY_BACKGROUND,
            AutoApply::Replace => AUTO_APPLY_REPLACE,
            AutoApply::Enabled => AUTO_APPLY_ENABLED,
            AutoApply::Off => AUTO_APPLY_OFF,
        };
        write!(f, "{value}")
    }
}

/// Biome to select
#[derive(Debug)]
pub enum BiomeArg {
    /// Main terrain will be selected.
    None,

    /// If default biome is specified it will be used, else main terrain.
    Default,

    /// Specified biome will be selected if exists.
    Some(String),
}

pub struct Terrain {
    name: String,
    default_biome: Option<String>,
    auto_apply: AutoApply,
    terrain: Biome,
    biomes: BTreeMap<String, Biome>,
}

impl Terrain {
    /// Converts [Terrain] into [Environment] by merging environment variables
    /// , aliases, and by appending constructors, destructors.
    pub fn into_environment(mut self, selected: BiomeArg) -> Result<Environment> {
        let selected = self.select_biome(selected)?;

        let merged = if let Some(biome) = selected {
            self.terrain.merged(biome)
        } else {
            self.terrain
        };

        Ok(Environment::new(
            self.name,
            self.default_biome,
            merged.name().to_string(),
            self.auto_apply,
            merged,
        ))
    }

    /// selects [Biome] and returns it by removing from biomes map
    ///
    /// if terrain is selected or is only one present returns [None]
    /// if selected biome does not exist returns [Err]
    fn select_biome(&mut self, selected: BiomeArg) -> Result<Option<Biome>> {
        match selected {
            BiomeArg::None => Ok(None),
            BiomeArg::Default => {
                if let Some(default_biome) = &self.default_biome {
                    if let Some(default) = self.biomes.remove(default_biome) {
                        Ok(Some(default))
                    } else {
                        bail!("the default biome {:?} does not exists", selected)
                    }
                } else {
                    Ok(None)
                }
            }
            BiomeArg::Some(selected) => {
                if let Some(biome) = self.biomes.remove(&selected) {
                    Ok(Some(biome))
                } else {
                    bail!("the biome {:?} does not exists", selected)
                }
            }
        }
    }

    pub fn example() -> Self {
        let terrain = Biome::example(NONE.to_string());

        let example_biome = Biome::new(
            EXAMPLE_BIOME.to_string(),
            example_biome_envs(),
            example_biome_aliases(),
            example_biome_constructors(),
            example_biome_destructors(),
        );

        let mut biomes: BTreeMap<String, Biome> = BTreeMap::new();
        biomes.insert(EXAMPLE_BIOME.to_string(), example_biome);

        Self {
            name: TERRAINIUM.to_string(),
            default_biome: Some(EXAMPLE_BIOME.to_string()),
            auto_apply: AutoApply::Off,
            terrain,
            biomes,
        }
    }
}

/// Environment variables used by `example_biome` in [Terrain] example
fn example_biome_envs() -> BTreeMap<String, String> {
    let mut envs: BTreeMap<String, String> = BTreeMap::new();
    envs.insert(EDITOR.to_string(), "nvim".to_string());
    envs.insert(ENV_VAR.to_string(), "overridden_env_val".to_string());
    envs
}

/// Aliases used by `example_biome` in [Terrain] example
fn example_biome_aliases() -> BTreeMap<String, String> {
    let mut aliases: BTreeMap<String, String> = BTreeMap::new();
    aliases.insert(
        TENTER.to_string(),
        "terrain enter --biome example_biome".to_string(),
    );
    aliases
}

/// Constructors used by `example_biome` in [Terrain] example
fn example_biome_constructors() -> Commands {
    Commands::new(
        vec![Command::new(
            "/bin/echo".to_string(),
            vec!["entering biome example_biome".to_string()],
            None,
        )],
        vec![Command::new(
            "/bin/bash".to_string(),
            vec![
                "-c".to_string(),
                "${PWD}/tests/scripts/print_num_for_10_sec".to_string(),
            ],
            None,
        )],
    )
}

/// Destructors used by `example_biome` in [Terrain] example
fn example_biome_destructors() -> Commands {
    Commands::new(
        vec![Command::new(
            "/bin/echo".to_string(),
            vec!["exiting biome example_biome".to_string()],
            None,
        )],
        vec![Command::new(
            "/bin/bash".to_string(),
            vec![
                "-c".to_string(),
                "${TERRAIN_DIR}/tests/scripts/print_num_for_10_sec".to_string(),
            ],
            None,
        )],
    )
}
