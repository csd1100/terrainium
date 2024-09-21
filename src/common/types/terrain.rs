use crate::common::types::biome::Biome;
use crate::common::types::command::Command;
use crate::common::types::commands::Commands;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Default)]
pub struct Terrain {
    terrain: Biome,
    biomes: BTreeMap<String, Biome>,
    default_biome: Option<String>,
}

impl Terrain {
    pub fn new(
        terrain: Biome,
        biomes: BTreeMap<String, Biome>,
        default_biome: Option<String>,
    ) -> Self {
        Terrain {
            terrain,
            biomes,
            default_biome,
        }
    }

    pub fn from_toml(toml_str: String) -> Result<Terrain> {
        toml::from_str(&toml_str).context("failed to parse terrain from toml")
    }

    pub fn to_toml(&self) -> Result<String> {
        toml::to_string(&self).context("failed to convert terrain to toml")
    }

    pub fn example() -> Self {
        let terrain = Biome::example();

        let mut envs: BTreeMap<String, String> = BTreeMap::new();
        envs.insert("EDITOR".to_string(), "nvim".to_string());

        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        aliases.insert(
            "tenter".to_string(),
            "terrainium enter -b example_biome".to_string(),
        );

        let constructors: Commands = Commands::new(
            vec![Command::new(
                "/bin/echo".to_string(),
                vec!["entering biome example_biome".to_string()],
            )],
            vec![],
        );

        let destructors: Commands = Commands::new(
            vec![Command::new(
                "/bin/echo".to_string(),
                vec!["exiting biome example_biome".to_string()],
            )],
            vec![],
        );

        let example_biome = Biome::new(envs, aliases, constructors, destructors);

        let mut biomes: BTreeMap<String, Biome> = BTreeMap::new();
        biomes.insert(String::from("example_biome"), example_biome);

        Terrain::new(terrain, biomes, Some(String::from("example_biome")))
    }
}
