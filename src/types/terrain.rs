use std::{
    collections::{hash_map::IntoIter, HashMap},
    path::Path,
};

use anyhow::{Context, Result};
#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    args::{BiomeArg, Pair},
    biomes::Biome,
    commands::{Command, Commands},
    errors::TerrainiumErrors,
    get::PrintableTerrain,
};

pub fn parse_terrain_from<P: AsRef<Path>>(a_toml_file: P) -> Result<Terrain> {
    let terrain = std::fs::read_to_string(a_toml_file).context("Unable to read file")?;
    let terrain: Terrain = toml::from_str(&terrain).context("Unable to parse terrain")?;
    Ok(terrain)
}

#[cfg_attr(feature = "terrain-schema", derive(JsonSchema))]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Terrain {
    #[serde(default = "schema_url", rename(serialize = "$schema"))]
    schema: String,

    terrain: Biome,
    default_biome: Option<String>,
    biomes: Option<HashMap<String, Biome>>,
}

impl Terrain {
    pub fn new() -> Terrain {
        Terrain {
            schema: schema_url(),
            terrain: Biome::new(),
            default_biome: None,
            biomes: None,
        }
    }

    pub fn to_toml(&self) -> Result<String> {
        toml::to_string(self).context("unable to convert terrain to toml")
    }

    pub fn get_biome(&self, biome: &String) -> Result<&Biome> {
        if let Some(biomes) = &self.biomes {
            if let Some(biome) = biomes.get(biome) {
                Ok(biome)
            } else {
                Err(TerrainiumErrors::BiomeNotFound(biome.to_owned()).into())
            }
        } else {
            Err(TerrainiumErrors::BiomesNotDefined.into())
        }
    }

    pub fn get_default_biome_name(&self) -> Result<String> {
        if let Some(default_biome) = &self.default_biome {
            let default_biome = default_biome.clone();
            Ok(default_biome)
        } else {
            Err(TerrainiumErrors::DefaultBiomeNotDefined.into())
        }
    }

    pub fn get_selected_biome_name(&self, selected: &Option<BiomeArg>) -> Result<String> {
        if let Some(selected) = selected {
            match selected {
                BiomeArg::None => Ok("none".to_string()),
                BiomeArg::Default => self.get_default_biome_name(),
                BiomeArg::Current(biome) => match self.get_biome(biome) {
                    Ok(_) => Ok(biome.to_string()),
                    Err(e) => Err(e),
                },
                BiomeArg::Value(biome) => match self.get_biome(biome) {
                    Ok(_) => Ok(biome.to_string()),
                    Err(e) => Err(e),
                },
            }
        } else if self.default_biome.is_some() {
            self.get_default_biome_name()
        } else {
            Ok("none".to_string())
        }
    }

    pub fn select_biome(&self, biome: Option<BiomeArg>) -> Result<&Biome> {
        let selected = self.get_selected_biome_name(&biome)?;
        if selected == *"none" {
            Ok(&self.terrain)
        } else {
            return self.get_biome(&selected);
        }
    }

    fn get_merged_biome(&self, biome: &Biome) -> Biome {
        self.terrain.merge(biome)
    }

    pub fn get_env(
        &self,
        selected: Option<BiomeArg>,
        env: Vec<String>,
    ) -> Result<HashMap<String, Option<String>>> {
        let environment = self.get(selected)?;
        environment.find_envs(env)
    }

    pub fn get_alias(
        &self,
        selected: Option<BiomeArg>,
        aliases: Vec<String>,
    ) -> Result<HashMap<String, Option<String>>> {
        let environment = self.get(selected)?;
        environment.find_aliases(aliases)
    }

    pub fn get(&self, selected: Option<BiomeArg>) -> Result<Biome> {
        let selected = self.select_biome(selected)?;
        if selected == &self.terrain {
            Ok(self.terrain.clone())
        } else {
            Ok(self.get_merged_biome(selected))
        }
    }

    pub fn get_printable_terrain(self, selected: Option<BiomeArg>) -> Result<PrintableTerrain> {
        let selected_name = self.get_selected_biome_name(&selected)?;
        Ok(PrintableTerrain {
            default_biome: self.default_biome.clone(),
            selected_biome: Some(selected_name),
            biome: self.get(selected)?,
            all: false,
        })
    }

    pub fn select_biome_mut(&mut self, biome: Option<BiomeArg>) -> Result<&mut Biome> {
        let selected = self.get_selected_biome_name(&biome)?;
        if selected == *"none" {
            Ok(&mut self.terrain)
        } else {
            return self.get_biome_mut(&selected);
        }
    }

    fn get_biome_mut(&mut self, biome: &String) -> Result<&mut Biome> {
        if let Some(biomes) = &mut self.biomes {
            if let Some(biome) = biomes.get_mut(biome) {
                Ok(biome)
            } else {
                Err(TerrainiumErrors::BiomeNotFound(biome.to_string()).into())
            }
        } else {
            Err(TerrainiumErrors::BiomesNotDefined.into())
        }
    }

    pub fn set_default_biome(&mut self, biome: String) -> Result<()> {
        let _ = self
            .get_biome(&biome)
            .context("unable to set default biome")?;
        self.default_biome = Some(biome);
        Ok(())
    }

    pub fn add_biome(&mut self, name: &String, biome: Biome) -> Result<()> {
        if self.get_biome(name).is_ok() {
            return Err(TerrainiumErrors::BiomeAlreadyExists(name.to_string()).into());
        }
        if self.biomes.is_none() {
            self.biomes = Some(HashMap::<String, Biome>::new());
        }
        self.biomes
            .as_mut()
            .expect("expect biomes to be initiated")
            .insert(name.to_string(), biome);
        Ok(())
    }

    pub fn update(
        &mut self,
        biome_arg: Option<BiomeArg>,
        envs: Option<Vec<Pair>>,
        aliases: Option<Vec<Pair>>,
    ) -> Result<()> {
        let biome_to_update: &mut Biome = self.select_biome_mut(biome_arg)?;
        if let Some(pairs) = envs {
            pairs
                .into_iter()
                .for_each(|pair| biome_to_update.update_env(pair.key, pair.value));
        }

        if let Some(pairs) = aliases {
            pairs
                .into_iter()
                .for_each(|pair| biome_to_update.update_alias(pair.key, pair.value));
        }
        Ok(())
    }

    pub fn add_and_update_biome(
        &mut self,
        biome_name: String,
        envs: Option<Vec<Pair>>,
        alias: Option<Vec<Pair>>,
    ) -> Result<()> {
        self.add_biome(&biome_name, Biome::new())
            .context("unable to create a new biome")?;
        self.update(Some(BiomeArg::Value(biome_name)), envs, alias)
            .context("failed to update newly created biome")?;
        Ok(())
    }

    pub fn example() -> Self {
        let main = Biome {
            constructors: Some(Commands {
                foreground: Some(vec![Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("entering terrain")]),
                }]),
                background: None,
            }),
            destructors: Some(Commands {
                foreground: Some(vec![Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("exiting terrain")]),
                }]),
                background: None,
            }),
            ..Biome::example()
        };

        let mut biomes = HashMap::<String, Biome>::new();
        let biome = Biome::example();
        let biome_name = String::from("example_biome");
        biomes.insert(biome_name.clone(), biome);

        Self {
            schema: schema_url(),
            terrain: main,
            default_biome: Some(biome_name),
            biomes: Some(biomes),
        }
    }
}

impl Default for Terrain {
    fn default() -> Self {
        Self::new()
    }
}

// Note: This IntoIterator returns merged biomes
impl IntoIterator for Terrain {
    type Item = (String, Biome);

    type IntoIter = IntoIter<String, Biome>;

    fn into_iter(self) -> Self::IntoIter {
        let mut iter = HashMap::<String, Biome>::new();
        iter.insert("none".to_string(), self.terrain.clone());
        if let Some(biomes) = self.biomes.as_ref() {
            biomes.iter().for_each(|(name, biome)| {
                iter.insert(name.to_string(), self.get_merged_biome(biome));
            });
        }

        iter.into_iter()
    }
}

pub fn schema_url() -> String {
    "https://raw.githubusercontent.com/csd1100/terrainium/main/schema/terrain-schema.json"
        .to_string()
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, path::PathBuf};

    use anyhow::{anyhow, Result};

    use crate::types::{
        args::{BiomeArg, Pair},
        biomes::Biome,
        commands::{Command, Commands},
        errors::TerrainiumErrors,
        terrain::test_data,
    };

    use super::{parse_terrain_from, Terrain};

    #[test]
    fn parse_toml_full() -> Result<()> {
        let expected = test_data::terrain_full();
        let parsed = parse_terrain_from(PathBuf::from("./example_configs/terrain.full.toml"))?;

        assert_eq!(expected, parsed);

        Ok(())
    }

    #[test]
    fn parse_toml_without_biomes() -> Result<()> {
        let expected = test_data::terrain_without_biomes();
        let parsed = parse_terrain_from(PathBuf::from(
            "./example_configs/terrain.without.biomes.toml",
        ))?;

        assert_eq!(expected, parsed);

        Ok(())
    }

    // #[test]
    // fn to_toml() -> Result<()> {
    // will fail because `toml` sometimes writes it in different order.
    //     let expected =
    //         std::fs::read_to_string(PathBuf::from("./example_configs/terrain.full.toml"))?;
    //     let parsed = get_full_terrain().to_toml()?;
    //
    //     assert_eq!(expected, parsed);
    //
    //     return Ok(());
    // }

    #[test]
    fn get_biome_returns_biome_if_present() -> Result<()> {
        let terrain = test_data::terrain_full();
        let expected = &test_data::biome("example_biome2", "nano");

        let actual = terrain.get_biome(&"example_biome2".to_owned())?;

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn get_biome_returns_error_if_not_present() -> Result<()> {
        let terrain = test_data::terrain_full();

        let non_existent = "non_existent".to_owned();
        let actual = terrain.get_biome(&non_existent).unwrap_err();

        assert_eq!(
            actual.to_string(),
            TerrainiumErrors::BiomeNotFound(non_existent).to_string()
        );

        Ok(())
    }

    #[test]
    fn get_biome_returns_error_if_biomes_not_defined() -> Result<()> {
        let terrain = test_data::terrain_without_biomes();

        let biome = "example_biome".to_owned();
        let actual = terrain.get_biome(&biome).unwrap_err();

        assert_eq!(
            actual.to_string(),
            TerrainiumErrors::BiomesNotDefined.to_string()
        );

        Ok(())
    }

    #[test]
    fn get_default_biome_name_returns_default_if_present() -> Result<()> {
        let terrain = test_data::terrain_full();

        let expected = "example_biome".to_owned();
        let actual = terrain.get_default_biome_name()?;

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn get_default_biome_name_returns_error_if_no_biomes() -> Result<()> {
        let terrain = test_data::terrain_without_biomes();

        let expected = TerrainiumErrors::DefaultBiomeNotDefined.to_string();
        let actual = terrain.get_default_biome_name().unwrap_err().to_string();

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn get_merged_biome_returns_merged_biome() -> Result<()> {
        let mut env = HashMap::<String, String>::new();
        env.insert(String::from("EDITOR"), String::from("nvim"));
        env.insert(String::from("TEST"), String::from("value"));
        let mut alias = HashMap::<String, String>::new();
        alias.insert(String::from("tedit"), String::from("terrainium edit"));
        alias.insert(
            String::from("tenter"),
            String::from("terrainium enter --biome example_biome"),
        );
        let constructor = Commands {
            foreground: Some(vec![
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("entering terrain")]),
                },
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("entering biome 'example_biome'")]),
                },
            ]),
            background: Some(vec![Command {
                exe: "run".to_string(),
                args: Some(vec!["something".to_string()]),
            }]),
        };
        let destructor = Commands {
            foreground: Some(vec![
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("exiting terrain")]),
                },
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("exiting biome 'example_biome'")]),
                },
            ]),
            background: Some(vec![Command {
                exe: "stop".to_string(),
                args: Some(vec!["something".to_string()]),
            }]),
        };
        let expected = Biome {
            env: Some(env),
            alias: Some(alias),
            constructors: Some(constructor),
            destructors: Some(destructor),
        };

        let terrain = test_data::terrain_full();

        let actual = terrain.get_merged_biome(terrain.get_biome(&"example_biome".to_string())?);

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn select_biome_returns_biome_if_biomearg_value() -> Result<()> {
        let terrain = test_data::terrain_full();
        let expected = &test_data::biome("example_biome2", "nano");

        let acutal = terrain.select_biome(Some(BiomeArg::Value("example_biome2".to_owned())))?;

        assert_eq!(expected, acutal);

        Ok(())
    }

    #[test]
    fn select_biome_returns_default_if_biomearg_default() -> Result<()> {
        let terrain = test_data::terrain_full();
        let expected = &test_data::biome("example_biome", "nvim");

        let acutal = terrain.select_biome(Some(BiomeArg::Default))?;

        assert_eq!(expected, acutal);

        Ok(())
    }

    #[test]
    fn select_biome_returns_main_terrain_if_biomearg_none() -> Result<()> {
        let terrain = test_data::terrain_full();
        let expected = &terrain.terrain;

        let acutal = terrain.select_biome(Some(BiomeArg::None))?;

        assert_eq!(expected, acutal);

        Ok(())
    }

    #[test]
    fn select_biome_returns_default_if_no_biome_arg() -> Result<()> {
        let terrain = test_data::terrain_full();
        let expected = &test_data::biome("example_biome", "nvim");

        let acutal = terrain.select_biome(None)?;

        assert_eq!(expected, acutal);

        Ok(())
    }

    #[test]
    fn select_biome_returns_main_terrain_if_no_biomearg_and_no_biomes_defined() -> Result<()> {
        let terrain = test_data::terrain_without_biomes();
        let expected = &terrain.terrain;

        let acutal = terrain.select_biome(None)?;

        assert_eq!(expected, acutal);

        Ok(())
    }

    #[test]
    fn select_biome_returns_err_if_biome_not_present() -> Result<()> {
        let terrain = test_data::terrain_full();
        let expected = TerrainiumErrors::BiomeNotFound("non_existent".to_owned()).to_string();

        let acutal = terrain
            .select_biome(Some(BiomeArg::Value("non_existent".to_owned())))
            .unwrap_err()
            .to_string();

        assert_eq!(expected, acutal);

        Ok(())
    }

    #[test]
    fn select_biome_returns_err_if_biomes_not_defined() -> Result<()> {
        let terrain = test_data::terrain_without_biomes();
        let expected = TerrainiumErrors::BiomesNotDefined.to_string();

        let acutal = terrain
            .select_biome(Some(BiomeArg::Value("non_existent".to_owned())))
            .unwrap_err()
            .to_string();

        assert_eq!(expected, acutal);

        Ok(())
    }

    #[test]
    fn select_biome_returns_err_if_biomes_not_defined_and_biomeargs_default_passed() -> Result<()> {
        let terrain = test_data::terrain_without_biomes();
        let expected = TerrainiumErrors::DefaultBiomeNotDefined.to_string();

        let acutal = terrain
            .select_biome(Some(BiomeArg::Default))
            .unwrap_err()
            .to_string();

        assert_eq!(expected, acutal);

        Ok(())
    }

    #[test]
    fn test_get_env() -> Result<()> {
        let terrain = test_data::terrain_full();

        let mut expected = HashMap::<String, Option<String>>::new();
        expected.insert("EDITOR".to_owned(), Some("vim".to_owned()));
        expected.insert("TEST".to_owned(), Some("value".to_owned()));
        expected.insert("VAR1".to_owned(), None);

        let to_find = vec![
            String::from("EDITOR"),
            String::from("VAR1"),
            String::from("TEST"),
        ];

        // test main terrain
        let actual = terrain.get_env(Some(BiomeArg::None), to_find.clone())?;
        assert_eq!(&expected, &actual);

        // test default biome
        expected.insert("EDITOR".to_owned(), Some("nvim".to_owned()));
        let actual = terrain.get_env(Some(BiomeArg::Default), to_find.clone())?;
        assert_eq!(&expected, &actual);

        // test `some` biome
        expected.insert("EDITOR".to_owned(), Some("nano".to_owned()));
        let actual = terrain.get_env(
            Some(BiomeArg::Value("example_biome2".to_owned())),
            to_find.clone(),
        )?;
        assert_eq!(&expected, &actual);

        Ok(())
    }

    #[test]
    fn get_env_throws_err_if_no_env_defined() -> Result<()> {
        let terrain = Terrain {
            schema: super::schema_url(),
            terrain: Biome {
                env: None,
                alias: None,
                constructors: None,
                destructors: None,
            },
            default_biome: None,
            biomes: None,
        };

        let to_find = vec![String::from("EDITOR"), String::from("VAR1")];

        let expected = TerrainiumErrors::EnvsNotDefined;
        // test main terrain
        let actual = terrain.get_env(Some(BiomeArg::None), to_find).unwrap_err();
        assert_eq!(&expected.to_string(), &actual.to_string());

        Ok(())
    }

    #[test]
    fn get_alias_throws_err_if_no_aliases_defined() -> Result<()> {
        let terrain = Terrain {
            schema: super::schema_url(),
            terrain: Biome {
                env: None,
                alias: None,
                constructors: None,
                destructors: None,
            },
            default_biome: None,
            biomes: None,
        };

        let to_find = vec![String::from("tedit"), String::from("VAR1")];

        let expected = TerrainiumErrors::AliasesNotDefined;
        // test main terrain
        let actual = terrain
            .get_alias(Some(BiomeArg::None), to_find)
            .unwrap_err();
        assert_eq!(&expected.to_string(), &actual.to_string());

        Ok(())
    }

    #[test]
    fn test_get_alias() -> Result<()> {
        let terrain = test_data::terrain_full();

        let to_find = vec![String::from("tenter"), String::from("ALIAS1")];

        let mut expected = HashMap::<String, Option<String>>::new();
        expected.insert("ALIAS1".to_owned(), None);

        // test main terrain
        expected.insert("tenter".to_owned(), Some("terrainium enter".to_owned()));
        let actual = terrain.get_alias(Some(BiomeArg::None), to_find.clone())?;
        assert_eq!(&expected, &actual);

        // test default biome
        expected.insert(
            "tenter".to_owned(),
            Some("terrainium enter --biome example_biome".to_owned()),
        );
        let actual = terrain.get_alias(Some(BiomeArg::Default), to_find.clone())?;
        assert_eq!(&expected, &actual);

        // test `some` biome
        expected.insert(
            "tenter".to_owned(),
            Some("terrainium enter --biome example_biome2".to_owned()),
        );
        let actual = terrain.get_alias(
            Some(BiomeArg::Value("example_biome2".to_owned())),
            to_find.clone(),
        )?;
        assert_eq!(&expected, &actual);

        Ok(())
    }

    #[test]
    fn terrain_get_returns_main_terrain_when_biomearg_none_passed() -> Result<()> {
        let terrain = test_data::terrain_full();
        let expected = test_data::terrain_full_main();

        let actual = terrain.get(Some(BiomeArg::None))?;

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn terrain_get_returns_merged_default_when_biomearg_default_passed() -> Result<()> {
        let terrain = test_data::terrain_full();
        let expected = test_data::merged_default();

        let actual = terrain.get(Some(BiomeArg::Default))?;

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn terrain_get_returns_merged_when_biomearg_value_passed() -> Result<()> {
        let terrain = test_data::terrain_full();
        let expected = test_data::merged_example_biome2();

        let actual = terrain.get(Some(BiomeArg::Value("example_biome2".to_string())))?;

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn terrain_get_returns_merged_default_when_biomearg_not_passed() -> Result<()> {
        let terrain = test_data::terrain_full();
        let expected = test_data::merged_default();

        let actual = terrain.get(None)?;

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn terrain_get_returns_main_terrain_when_biomearg_not_passed_and_no_default() -> Result<()> {
        let terrain = test_data::terrain_without_biomes();
        let expected = test_data::terrain_without_biomes_main();

        let actual = terrain.get(None)?;

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn terrain_get_returns_error_when_biomearg_default_passed_but_not_defined() -> Result<()> {
        let terrain = test_data::terrain_without_biomes();
        let expected = TerrainiumErrors::DefaultBiomeNotDefined.to_string();

        let actual = terrain
            .get(Some(BiomeArg::Default))
            .unwrap_err()
            .to_string();

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn terrain_get_returns_error_when_biomearg_value_passed_but_not_defined() -> Result<()> {
        let terrain = test_data::terrain_full();
        let expected = TerrainiumErrors::BiomeNotFound("non_existent".to_string()).to_string();

        let actual = terrain
            .get(Some(BiomeArg::Value("non_existent".to_string())))
            .unwrap_err()
            .to_string();

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn set_default_biome_works() -> Result<()> {
        let mut expected = test_data::terrain_full();
        expected.default_biome = Some("example_biome2".to_string());

        let mut terrain = test_data::terrain_full();
        terrain.set_default_biome("example_biome2".to_string())?;
        let actual = terrain;

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn update_default_biome_returns_error_if_not_found() -> Result<()> {
        let expected = "unable to set default biome".to_string();

        let mut terrain = test_data::terrain_full();
        let actual = terrain
            .set_default_biome("non_existent".to_string())
            .unwrap_err()
            .to_string();

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn add_biome_works() -> Result<()> {
        let mut expected = test_data::terrain_full();
        if let Some(biomes) = &mut expected.biomes {
            biomes.insert(
                "new_biome".to_string(),
                Biome {
                    env: Some(HashMap::<String, String>::new()),
                    alias: Some(HashMap::<String, String>::new()),
                    constructors: None,
                    destructors: None,
                },
            );
        }

        let mut terrain = test_data::terrain_full();
        terrain.add_biome(&"new_biome".to_string(), Biome::new())?;

        assert_eq!(expected, terrain);

        Ok(())
    }

    #[test]
    fn add_biome_works_when_none_present() -> Result<()> {
        let mut expected = test_data::terrain_without_biomes();
        expected.biomes = Some(HashMap::<String, Biome>::new());
        if let Some(biomes) = &mut expected.biomes {
            biomes.insert(
                "new_biome".to_string(),
                Biome {
                    env: Some(HashMap::<String, String>::new()),
                    alias: Some(HashMap::<String, String>::new()),
                    constructors: None,
                    destructors: None,
                },
            );
        }

        let mut terrain = test_data::terrain_without_biomes();
        terrain.add_biome(&"new_biome".to_string(), Biome::new())?;

        assert_eq!(expected, terrain);

        Ok(())
    }

    #[test]
    fn add_biome_fails_if_already_present() -> Result<()> {
        let expected =
            TerrainiumErrors::BiomeAlreadyExists("example_biome2".to_string()).to_string();

        let mut terrain = test_data::terrain_full();
        let actual = terrain
            .add_biome(&"example_biome2".to_string(), Biome::new())
            .unwrap_err()
            .to_string();

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn get_biome_mut_returns_biome_if_present() -> Result<()> {
        let mut terrain = test_data::terrain_full();
        let expected = &mut test_data::biome("example_biome2", "nano");

        let actual = terrain.get_biome_mut(&"example_biome2".to_owned())?;

        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn get_biome_mut_returns_error_if_not_present() -> Result<()> {
        let mut terrain = test_data::terrain_full();

        let non_existent = "non_existent".to_owned();
        let actual = terrain.get_biome_mut(&non_existent).unwrap_err();

        assert_eq!(
            actual.to_string(),
            TerrainiumErrors::BiomeNotFound(non_existent).to_string()
        );

        Ok(())
    }

    #[test]
    fn get_biome_mut_returns_error_if_biomes_not_defined() -> Result<()> {
        let mut terrain = test_data::terrain_without_biomes();

        let biome = "example_biome".to_owned();
        let actual = terrain.get_biome_mut(&biome).unwrap_err();

        assert_eq!(
            actual.to_string(),
            TerrainiumErrors::BiomesNotDefined.to_string()
        );

        Ok(())
    }

    #[test]
    fn update_works_full() -> Result<()> {
        let mut terrain = test_data::terrain_full();
        let biome_args = BiomeArg::Value("example_biome2".to_owned());
        let env = vec![
            Pair {
                key: "EDITOR".to_string(),
                value: "nvim".to_string(),
            },
            Pair {
                key: "NEW".to_string(),
                value: "new".to_string(),
            },
        ];
        let alias = vec![
            Pair {
                key: "new_alias".to_string(),
                value: "new value".to_string(),
            },
            Pair {
                key: "tenter".to_string(),
                value: "terrainium enter".to_string(),
            },
        ];

        let mut expected = test_data::terrain_full();
        if let Some(biomes) = expected.biomes.as_mut() {
            let to_update = biomes.get_mut(&"example_biome2".to_string());
            if let Some(to_update) = to_update {
                if let Some(env) = to_update.env.as_mut() {
                    env.insert("EDITOR".to_string(), "nvim".to_string());
                    env.insert("NEW".to_string(), "new".to_string());
                }
                if let Some(aliases) = to_update.alias.as_mut() {
                    aliases.insert("tenter".to_string(), "terrainium enter".to_string());
                    aliases.insert("new_alias".to_string(), "new value".to_string());
                }
            } else {
                return Err(anyhow!("Expected to be present"));
            }
        }

        terrain.update(Some(biome_args), Some(env), Some(alias))?;

        assert_eq!(expected, terrain);

        Ok(())
    }

    #[test]
    fn update_works_only_env() -> Result<()> {
        let mut terrain = test_data::terrain_full();
        let biome_args = BiomeArg::Value("example_biome2".to_owned());
        let env = vec![
            Pair {
                key: "EDITOR".to_string(),
                value: "nvim".to_string(),
            },
            Pair {
                key: "NEW".to_string(),
                value: "new".to_string(),
            },
        ];
        let mut expected = test_data::terrain_full();
        if let Some(biomes) = expected.biomes.as_mut() {
            let to_update = biomes.get_mut(&"example_biome2".to_string());
            if let Some(to_update) = to_update {
                if let Some(env) = to_update.env.as_mut() {
                    env.insert("EDITOR".to_string(), "nvim".to_string());
                    env.insert("NEW".to_string(), "new".to_string());
                }
            } else {
                return Err(anyhow!("Expected to be present"));
            }
        }

        terrain.update(Some(biome_args), Some(env), None)?;

        assert_eq!(expected, terrain);

        Ok(())
    }

    #[test]
    fn update_works_only_alias() -> Result<()> {
        let mut terrain = test_data::terrain_full();
        let biome_args = BiomeArg::Value("example_biome2".to_owned());
        let alias = vec![
            Pair {
                key: "new_alias".to_string(),
                value: "new value".to_string(),
            },
            Pair {
                key: "tenter".to_string(),
                value: "terrainium enter".to_string(),
            },
        ];

        let mut expected = test_data::terrain_full();
        if let Some(biomes) = expected.biomes.as_mut() {
            let to_update = biomes.get_mut(&"example_biome2".to_string());
            if let Some(to_update) = to_update {
                if let Some(aliases) = to_update.alias.as_mut() {
                    aliases.insert("tenter".to_string(), "terrainium enter".to_string());
                    aliases.insert("new_alias".to_string(), "new value".to_string());
                }
            } else {
                return Err(anyhow!("Expected to be present"));
            }
        }

        terrain.update(Some(biome_args), None, Some(alias))?;

        assert_eq!(expected, terrain);

        Ok(())
    }

    #[test]
    fn update_works_main_terrain() -> Result<()> {
        let mut terrain = test_data::terrain_full();
        let biome_args = BiomeArg::None;
        let alias = vec![
            Pair {
                key: "new_alias".to_string(),
                value: "new value".to_string(),
            },
            Pair {
                key: "tenter".to_string(),
                value: "terrainium enter".to_string(),
            },
        ];

        let env = vec![
            Pair {
                key: "EDITOR".to_string(),
                value: "nvim".to_string(),
            },
            Pair {
                key: "NEW".to_string(),
                value: "new".to_string(),
            },
        ];

        let mut expected = test_data::terrain_full();
        if let Some(aliases) = expected.terrain.alias.as_mut() {
            aliases.insert("tenter".to_string(), "terrainium enter".to_string());
            aliases.insert("new_alias".to_string(), "new value".to_string());
        }
        if let Some(env) = expected.terrain.env.as_mut() {
            env.insert("EDITOR".to_string(), "nvim".to_string());
            env.insert("NEW".to_string(), "new".to_string());
        }

        terrain.update(Some(biome_args), Some(env), Some(alias))?;

        assert_eq!(expected, terrain);

        Ok(())
    }

    #[test]
    fn update_updates_nothing_if_nothing_passed() -> Result<()> {
        let mut terrain = test_data::terrain_full();
        let biome_args = BiomeArg::None;
        let expected = test_data::terrain_full();
        terrain.update(Some(biome_args), None, None)?;

        assert_eq!(expected, terrain);

        Ok(())
    }

    #[test]
    fn create_and_update_new_biome() -> Result<()> {
        // setup
        let mut env = HashMap::<String, String>::new();
        env.insert("EDITOR".to_string(), "nvim".to_string());
        env.insert("TEST".to_string(), "test".to_string());
        let mut alias = HashMap::<String, String>::new();
        alias.insert(
            "tenter".to_string(),
            "terrainium enter --biome name".to_string(),
        );
        alias.insert("alias1".to_string(), "alias1".to_string());
        let constructor = Commands {
            foreground: Some(vec![Command {
                exe: String::from("echo"),
                args: Some(vec![String::from("entering biome 'name'")]),
            }]),
            background: None,
        };
        let destructor = Commands {
            foreground: Some(vec![Command {
                exe: String::from("echo"),
                args: Some(vec![String::from("exiting biome 'name'")]),
            }]),
            background: None,
        };
        let biome = Biome {
            env: Some(env),
            alias: Some(alias),
            constructors: Some(constructor),
            destructors: Some(destructor),
        };
        let mut expected = test_data::terrain_without_biomes();
        expected.default_biome = Some("name".to_string());
        expected.biomes = Some(HashMap::<String, Biome>::new());
        if let Some(biomes) = expected.biomes.as_mut() {
            biomes.insert("name".to_string(), biome);
        } else {
            return Err(anyhow!("expected to be present"));
        }

        // test
        let mut terrain = test_data::terrain_without_biomes();
        let biome_args = Some(BiomeArg::Value("name".to_string()));
        let biome = test_data::biome("name", "editor");
        let env = Some(vec![
            Pair {
                key: "EDITOR".to_string(),
                value: "nvim".to_string(),
            },
            Pair {
                key: "TEST".to_string(),
                value: "test".to_string(),
            },
        ]);
        let alias = Some(vec![
            Pair {
                key: "alias1".to_string(),
                value: "alias1".to_string(),
            },
            Pair {
                key: "tenter".to_string(),
                value: "terrainium enter --biome name".to_string(),
            },
        ]);
        terrain.add_biome(&"name".to_string(), biome)?;
        terrain.set_default_biome("name".to_string())?;
        terrain.update(biome_args, env, alias)?;

        // assertion
        assert_eq!(expected, terrain);

        Ok(())
    }
}

pub mod test_data {
    use std::collections::HashMap;

    use crate::types::{
        biomes::Biome,
        commands::{Command, Commands},
    };

    use super::Terrain;

    pub fn biome(name: &str, editor: &str) -> Biome {
        let name = name.to_owned();
        let editor = editor.to_owned();
        let mut env = HashMap::<String, String>::new();
        env.insert(String::from("EDITOR"), editor);
        let mut alias = HashMap::<String, String>::new();
        alias.insert(
            String::from("tenter"),
            String::from("terrainium enter --biome ") + &name,
        );
        let constructor = Commands {
            foreground: Some(vec![Command {
                exe: String::from("echo"),
                args: Some(vec![String::from("entering biome '") + &name + "'"]),
            }]),
            background: None,
        };
        let destructor = Commands {
            foreground: Some(vec![Command {
                exe: String::from("echo"),
                args: Some(vec![String::from("exiting biome '") + &name + "'"]),
            }]),
            background: None,
        };

        Biome {
            env: Some(env),
            alias: Some(alias),
            constructors: Some(constructor),
            destructors: Some(destructor),
        }
    }

    pub fn terrain_full_main() -> Biome {
        terrain_full().terrain
    }

    pub fn merged_default() -> Biome {
        let mut env = HashMap::<String, String>::new();
        env.insert(String::from("EDITOR"), String::from("nvim"));
        env.insert(String::from("TEST"), String::from("value"));
        let mut alias = HashMap::<String, String>::new();
        alias.insert(String::from("tedit"), String::from("terrainium edit"));
        alias.insert(
            String::from("tenter"),
            String::from("terrainium enter --biome example_biome"),
        );
        let constructor = Commands {
            foreground: Some(vec![
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("entering terrain")]),
                },
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("entering biome 'example_biome'")]),
                },
            ]),
            background: Some(vec![Command {
                exe: "run".to_string(),
                args: Some(vec!["something".to_string()]),
            }]),
        };
        let destructor = Commands {
            foreground: Some(vec![
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("exiting terrain")]),
                },
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("exiting biome 'example_biome'")]),
                },
            ]),
            background: Some(vec![Command {
                exe: "stop".to_string(),
                args: Some(vec!["something".to_string()]),
            }]),
        };
        Biome {
            env: Some(env),
            alias: Some(alias),
            constructors: Some(constructor),
            destructors: Some(destructor),
        }
    }

    pub fn merged_example_biome2() -> Biome {
        let mut env = HashMap::<String, String>::new();
        env.insert(String::from("EDITOR"), String::from("nano"));
        env.insert(String::from("TEST"), String::from("value"));
        let mut alias = HashMap::<String, String>::new();
        alias.insert(String::from("tedit"), String::from("terrainium edit"));
        alias.insert(
            String::from("tenter"),
            String::from("terrainium enter --biome example_biome2"),
        );
        let constructor = Commands {
            foreground: Some(vec![
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("entering terrain")]),
                },
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("entering biome 'example_biome2'")]),
                },
            ]),
            background: Some(vec![Command {
                exe: "run".to_string(),
                args: Some(vec!["something".to_string()]),
            }]),
        };
        let destructor = Commands {
            foreground: Some(vec![
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("exiting terrain")]),
                },
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("exiting biome 'example_biome2'")]),
                },
            ]),
            background: Some(vec![Command {
                exe: "stop".to_string(),
                args: Some(vec!["something".to_string()]),
            }]),
        };
        Biome {
            env: Some(env),
            alias: Some(alias),
            constructors: Some(constructor),
            destructors: Some(destructor),
        }
    }

    pub fn terrain_full() -> Terrain {
        let mut env = HashMap::<String, String>::new();
        env.insert(String::from("EDITOR"), String::from("vim"));
        env.insert(String::from("TEST"), String::from("value"));
        let mut alias = HashMap::<String, String>::new();
        alias.insert(String::from("tedit"), String::from("terrainium edit"));
        alias.insert(String::from("tenter"), String::from("terrainium enter"));
        let constructor = Commands {
            foreground: Some(vec![Command {
                exe: String::from("echo"),
                args: Some(vec![String::from("entering terrain")]),
            }]),
            background: Some(vec![Command {
                exe: String::from("run"),
                args: Some(vec![String::from("something")]),
            }]),
        };
        let destructor = Commands {
            foreground: Some(vec![Command {
                exe: String::from("echo"),
                args: Some(vec![String::from("exiting terrain")]),
            }]),
            background: Some(vec![Command {
                exe: String::from("stop"),
                args: Some(vec![String::from("something")]),
            }]),
        };

        let biome1_name = "example_biome";
        let biome1 = biome(biome1_name, "nvim");
        let biome2_name = "example_biome2";
        let biome2 = biome(biome2_name, "nano");
        let mut biomes = HashMap::<String, Biome>::new();
        biomes.insert(biome1_name.to_owned(), biome1);
        biomes.insert(biome2_name.to_owned(), biome2);

        Terrain {
            schema: super::schema_url(),
            terrain: Biome {
                env: Some(env),
                alias: Some(alias),
                constructors: Some(constructor),
                destructors: Some(destructor),
            },
            default_biome: Some("example_biome".to_owned()),
            biomes: Some(biomes),
        }
    }

    pub fn terrain_without_biomes_main() -> Biome {
        terrain_without_biomes().terrain
    }

    pub fn terrain_without_biomes() -> Terrain {
        let mut env = HashMap::<String, String>::new();
        env.insert(String::from("VAR1"), String::from("val1"));
        env.insert(String::from("VAR2"), String::from("val2"));
        env.insert(String::from("VAR3"), String::from("val3"));
        let mut alias = HashMap::<String, String>::new();
        alias.insert(String::from("alias1"), String::from("run1"));
        alias.insert(String::from("alias2"), String::from("run2"));
        alias.insert(String::from("alias3"), String::from("run3"));
        let constructor = Commands {
            foreground: Some(vec![Command {
                exe: String::from("run1"),
                args: Some(vec![String::from("something-new")]),
            }]),
            background: None,
        };
        let destructor = Commands {
            foreground: Some(vec![Command {
                exe: String::from("stop1"),
                args: Some(vec![String::from("something-old")]),
            }]),
            background: None,
        };
        Terrain {
            schema: super::schema_url(),
            terrain: Biome {
                env: Some(env),
                alias: Some(alias),
                constructors: Some(constructor),
                destructors: Some(destructor),
            },
            default_biome: None,
            biomes: None,
        }
    }
}
