use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::types::errors::TerrainiumErrors;

use super::{aliases::KeyValue, biomes::Biome, commands::Commands};

pub fn parse_terrain(path: PathBuf) -> Result<Terrain> {
    let terrain = std::fs::read_to_string(path).context("Unable to read file")?;
    let terrain: Terrain = toml::from_str(&terrain).context("Unable to parse terrain")?;
    return Ok(terrain);
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Terrain {
    pub env: Option<KeyValue>,
    pub aliases: Option<KeyValue>,
    pub construct: Option<Commands>,
    pub deconstruct: Option<Commands>,
    pub default_biome: Option<String>,
    pub biomes: Option<Vec<Biome>>,

    #[serde(skip_serializing, skip_deserializing)]
    pub selected_biome: String,
}

impl Terrain {
    pub fn get_biome(&self, biome: &String) -> Result<Option<&Biome>> {
        if let Some(biomes) = &self.biomes {
            let found = biomes.iter().find(|b| &b.name == biome);
            if found.is_some() {
                return Ok(found);
            } else {
                return Err(anyhow!(TerrainiumErrors::BiomeNotFound(biome.to_string())));
            }
        }
        return Ok(None);
    }

    pub fn select_biome(&mut self, biome: Option<String>) -> Result<()> {
        if let Some(select) = biome {
            if let Some(_) = self.get_biome(&select)? {
                self.selected_biome = select;
            }
        } else {
            if let Some(default) = &self.default_biome {
                self.selected_biome = String::from(default);
            }
        }
        return Ok(());
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;

    use crate::types::{biomes::Biome, errors::TerrainiumErrors};

    use super::Terrain;

    #[test]
    fn selected_biome_empty_when_no_biome_passed() -> Result<()> {
        let mut test_data = Terrain {
            env: None,
            aliases: None,
            construct: None,
            deconstruct: None,
            default_biome: None,
            biomes: Some(vec![]),
            selected_biome: String::from(""),
        };

        test_data.select_biome(None)?;

        assert_eq!(test_data.selected_biome, String::from(""));

        return Ok(());
    }

    #[test]
    fn error_when_biome_passed_but_not_present() -> Result<()> {
        let mut test_data = Terrain {
            env: None,
            aliases: None,
            construct: None,
            deconstruct: None,
            default_biome: None,
            biomes: Some(vec![]),
            selected_biome: String::from(""),
        };

        let err = test_data
            .select_biome(Some(String::from("value")))
            .unwrap_err();

        assert_eq!(
            err.to_string(),
            TerrainiumErrors::BiomeNotFound(String::from("value")).to_string()
        );

        return Ok(());
    }

    #[test]
    fn selected_biome_when_biome_passed() -> Result<()> {
        let mut test_data = Terrain {
            env: None,
            aliases: None,
            construct: None,
            deconstruct: None,
            default_biome: None,
            biomes: Some(vec![
                Biome {
                    name: String::from("one"),
                    env: None,
                    aliases: None,
                    construct: None,
                    deconstruct: None,
                },
                Biome {
                    name: String::from("two"),
                    env: None,
                    aliases: None,
                    construct: None,
                    deconstruct: None,
                },
            ]),
            selected_biome: String::from(""),
        };

        test_data.select_biome(Some(String::from("one")))?;

        assert_eq!(test_data.selected_biome, String::from("one"));

        return Ok(());
    }

    #[test]
    fn selected_biome_when_none_passed() -> Result<()> {
        let mut test_data = Terrain {
            env: None,
            aliases: None,
            construct: None,
            deconstruct: None,
            default_biome: Some(String::from("two")),
            biomes: Some(vec![
                Biome {
                    name: String::from("one"),
                    env: None,
                    aliases: None,
                    construct: None,
                    deconstruct: None,
                },
                Biome {
                    name: String::from("two"),
                    env: None,
                    aliases: None,
                    construct: None,
                    deconstruct: None,
                },
            ]),
            selected_biome: String::from(""),
        };

        test_data.select_biome(None)?;

        assert_eq!(test_data.selected_biome, String::from("two"));

        return Ok(());
    }
}
