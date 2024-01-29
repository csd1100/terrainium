use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::{
    biomes::Biome,
    commands::{Command, Commands},
};

pub fn parse_terrain(path: PathBuf) -> Result<Terrain> {
    let terrain = std::fs::read_to_string(path).context("Unable to read file")?;
    let terrain: Terrain = toml::from_str(&terrain).context("Unable to parse terrain")?;
    return Ok(terrain);
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Terrain {
    terrain: Biome,
    default_biome: Option<String>,
    biomes: Option<HashMap<String, Biome>>,
}

impl Terrain {
    pub fn new() -> Terrain {
        return Terrain {
            terrain: Biome::new(),
            default_biome: None,
            biomes: None,
        };
    }

    pub fn to_toml(&self) -> Result<String> {
        return Ok(toml::to_string(self).context("unable to convert terrain to toml")?);
    }
}

impl Default for Terrain {
    fn default() -> Self {
        let mut main = Biome::default();
        main.constructor = Some(Commands {
            exec: vec![Command {
                exe: String::from("echo"),
                args: Some(vec![String::from("entering terrain")]),
            }],
        });
        main.destructor = Some(Commands {
            exec: vec![Command {
                exe: String::from("echo"),
                args: Some(vec![String::from("exiting terrain")]),
            }],
        });

        let mut biomes = HashMap::<String, Biome>::new();
        let biome = Biome::default();
        let biome_name = String::from("example_biome");
        biomes.insert(biome_name.clone(), biome);

        Self {
            terrain: main,
            default_biome: Some(biome_name),
            biomes: Some(biomes),
        }
    }
}
