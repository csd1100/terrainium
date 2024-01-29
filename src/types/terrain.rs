use std::{collections::HashMap, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use super::{
    args::{BiomeArg, Pair},
    biomes::Biome,
    commands::{Command, Commands},
};

pub fn parse_terrain(path: &PathBuf) -> Result<Terrain> {
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

    pub fn get_biome(&self, biome: &String) -> Result<Option<&Biome>> {
        if let Some(biomes) = &self.biomes {
            if let Some(biome) = biomes.get(biome) {
                return Ok(Some(biome));
            } else {
                return Err(anyhow!(format!("biome {} is not defined", biome)));
            }
        } else {
            return Err(anyhow!("biomes are not defined"));
        }
    }

    pub fn update_default_biome(&mut self, biome: String) -> Result<()> {
        if let Some(_) = self.get_biome(&biome)? {
            self.default_biome = Some(biome);
        }
        return Ok(());
    }

    pub fn add_biome(&mut self, name: &String, biome: Biome) -> Result<()> {
        if let Ok(_) = self.get_biome(name) {
            return Err(anyhow!(format!("biome {} already exists", name)));
        }
        if let None = self.biomes {
            self.biomes = Some(HashMap::<String, Biome>::new());
        }
        self.biomes
            .as_mut()
            .expect("expect biomes to be initiated")
            .insert(name.to_string(), biome);
        return Ok(());
    }

    fn get_biome_mut(&mut self, biome: &String) -> Result<&mut Biome> {
        if let Some(biomes) = &mut self.biomes {
            if let Some(biome) = biomes.get_mut(biome) {
                return Ok(biome);
            } else {
                return Err(anyhow!(format!("biome {} is not defined", biome)));
            }
        } else {
            return Err(anyhow!("biomes are not defined"));
        }
    }

    fn get_default_biome_mut(&mut self) -> Result<&mut Biome> {
        if let Some(default_biome) = &self.default_biome {
            let default_biome = default_biome.clone();
            return Ok(self.get_biome_mut(&default_biome)?);
        } else {
            return Err(anyhow!("default biome not set"));
        }
    }

    fn get_terrain_mut(&mut self) -> &mut Biome {
        return &mut self.terrain;
    }

    pub fn update(
        &mut self,
        biome_args: Option<BiomeArg>,
        env: Option<Vec<Pair>>,
        alias: Option<Vec<Pair>>,
    ) -> Result<()> {
        let biome_to_update: &mut Biome;
        if let Some(biome_args) = biome_args {
            match biome_args {
                BiomeArg::Default => {
                    biome_to_update = self.get_default_biome_mut()?;
                }
                BiomeArg::None => {
                    biome_to_update = self.get_terrain_mut();
                }
                BiomeArg::Value(biome) => {
                    biome_to_update = self.get_biome_mut(&biome)?;
                }
            }
        } else {
            if let Ok(default) = self.get_default_biome_mut() {
                biome_to_update = default;
            } else {
                biome_to_update = self.get_terrain_mut();
            }
        }
        if let Some(pairs) = env {
            pairs
                .into_iter()
                .for_each(|pair| biome_to_update.update_env(pair.key, pair.value));
        }

        if let Some(pairs) = alias {
            pairs
                .into_iter()
                .for_each(|pair| biome_to_update.update_alias(pair.key, pair.value));
        }
        return Ok(());
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
