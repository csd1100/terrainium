use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::types::errors::TerrainiumErrors;

use super::{
    aliases::KeyValue,
    biomes::Biome,
    commands::{Command, Commands},
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Terrain {
    pub env: Option<KeyValue>,
    pub aliases: Option<KeyValue>,
    pub construct: Option<Commands>,
    pub destruct: Option<Commands>,
    pub default_biome: Option<String>,
    pub biomes: Option<Vec<Biome>>,

    #[serde(skip_serializing, skip_deserializing)]
    pub selected_biome: String,
}

impl Terrain {
    pub fn new() -> Terrain {
        return Terrain {
            env: Some(KeyValue::new()),
            aliases: Some(KeyValue::new()),
            construct: None,
            destruct: None,
            default_biome: Some(String::new()),
            biomes: Some(vec![Biome::new()]),
            selected_biome: String::new(),
        };
    }

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

impl Default for Terrain {
    fn default() -> Self {
        let mut env_vars = KeyValue::new();
        env_vars.insert(String::from("EDITOR"), String::from("vim"));
        let mut aliases = KeyValue::new();
        aliases.insert(String::from("tenter"), String::from("terrain enter"));
        aliases.insert(String::from("tedit"), String::from("terrain edit"));
        let construct = Commands {
            exec: vec![Command {
                exe: String::from("echo"),
                args: Some(vec!["entering terrain".to_string()]),
            }],
        };
        let destruct = Commands {
            exec: vec![Command {
                exe: String::from("echo"),
                args: Some(vec!["exiting terrain".to_string()]),
            }],
        };
        let biome = Biome::default();
        let biome_name = String::from(&biome.name);
        return Terrain {
            env: Some(env_vars),
            aliases: Some(aliases),
            construct: Some(construct),
            destruct: Some(destruct),
            default_biome: Some(biome_name.clone()),
            biomes: Some(vec![biome]),
            selected_biome: biome_name,
        };
    }
}

pub fn parse_terrain(path: PathBuf) -> Result<Terrain> {
    let terrain = std::fs::read_to_string(path).context("Unable to read file")?;
    let terrain: Terrain = toml::from_str(&terrain).context("Unable to parse terrain")?;
    return Ok(terrain);
}

pub fn terrain_to_toml(terrain: Terrain) -> Result<String> {
    return Ok(toml::to_string_pretty(&terrain).context("unable to convert terrain to toml")?);
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
            destruct: None,
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
            destruct: None,
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
            destruct: None,
            default_biome: None,
            biomes: Some(vec![
                Biome {
                    name: String::from("one"),
                    env: None,
                    aliases: None,
                    construct: None,
                    destruct: None,
                },
                Biome {
                    name: String::from("two"),
                    env: None,
                    aliases: None,
                    construct: None,
                    destruct: None,
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
            destruct: None,
            default_biome: Some(String::from("two")),
            biomes: Some(vec![
                Biome {
                    name: String::from("one"),
                    env: None,
                    aliases: None,
                    construct: None,
                    destruct: None,
                },
                Biome {
                    name: String::from("two"),
                    env: None,
                    aliases: None,
                    construct: None,
                    destruct: None,
                },
            ]),
            selected_biome: String::from(""),
        };

        test_data.select_biome(None)?;

        assert_eq!(test_data.selected_biome, String::from("two"));

        return Ok(());
    }
}
