use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::{
    args::{BiomeArg, Pair},
    biomes::Biome,
    commands::{Command, Commands},
    errors::TerrainiumErrors,
};

pub fn parse_terrain(path: &PathBuf) -> Result<Terrain> {
    let terrain = std::fs::read_to_string(path).context("Unable to read file")?;
    let terrain: Terrain = toml::from_str(&terrain).context("Unable to parse terrain")?;
    return Ok(terrain);
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

    pub fn get_biome(&self, biome: &String) -> Result<&Biome> {
        if let Some(biomes) = &self.biomes {
            if let Some(biome) = biomes.get(biome) {
                return Ok(biome);
            } else {
                return Err(TerrainiumErrors::BiomeNotFound(biome.to_owned()).into());
            }
        } else {
            return Err(TerrainiumErrors::BiomesNotDefined.into());
        }
    }

    pub fn get_default_biome_name(&self) -> Result<String> {
        if let Some(default_biome) = &self.default_biome {
            let default_biome = default_biome.clone();
            return Ok(default_biome);
        } else {
            return Err(TerrainiumErrors::DefaultBiomeNotDefined.into());
        }
    }

    pub fn select_biome(&self, biome: Option<BiomeArg>) -> Result<&Biome> {
        if let Some(selected) = biome {
            match selected {
                BiomeArg::Value(biome) => return self.get_biome(&biome),
                BiomeArg::None => return Ok(&self.terrain),
                BiomeArg::Default => return self.get_biome(&self.get_default_biome_name()?),
            }
        } else {
            if let Some(_) = self.default_biome {
                return self.get_biome(&self.get_default_biome_name()?);
            } else {
                return Ok(&self.terrain);
            }
        }
    }

    fn get_merged_biome(&self, biome: &Biome) -> Biome {
        return self.terrain.merge(biome);
    }

    pub fn get_env(
        &self,
        selected: Option<BiomeArg>,
        env: Vec<String>,
    ) -> Result<HashMap<String, Option<String>>> {
        let environment = self.get(selected)?;
        return environment.find_envs(env);
    }

    pub fn get_alias(
        &self,
        selected: Option<BiomeArg>,
        aliases: Vec<String>,
    ) -> Result<HashMap<String, Option<String>>> {
        let environment = self.get(selected)?;
        return environment.find_aliases(aliases);
    }

    pub fn get(&self, selected: Option<BiomeArg>) -> Result<Biome> {
        let selected = self.select_biome(selected)?;
        if selected == &self.terrain {
            return Ok(self.terrain.clone());
        } else {
            return Ok(self.get_merged_biome(selected));
        }
    }

    pub fn select_biome_mut(&mut self, biome: Option<BiomeArg>) -> Result<&mut Biome> {
        if let Some(selected) = biome {
            match selected {
                BiomeArg::Value(biome) => return self.get_biome_mut(&biome),
                BiomeArg::None => return Ok(&mut self.terrain),
                BiomeArg::Default => return self.get_default_biome_mut(),
            }
        } else {
            if let Some(_) = self.default_biome {
                return self.get_default_biome_mut();
            } else {
                return Ok(&mut self.terrain);
            }
        }
    }

    pub fn update_default_biome(&mut self, biome: String) -> Result<()> {
        if let Ok(_) = self.get_biome(&biome) {
            self.default_biome = Some(biome);
        } else {
            return Err(TerrainiumErrors::BiomeNotFound(biome).into());
        }
        return Ok(());
    }

    pub fn add_biome(&mut self, name: &String, biome: Biome) -> Result<()> {
        if let Ok(_) = self.get_biome(name) {
            return Err(TerrainiumErrors::BiomeAlreadyExists(name.to_string()).into());
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
                return Err(TerrainiumErrors::BiomeNotFound(biome.to_string()).into());
            }
        } else {
            return Err(TerrainiumErrors::BiomesNotDefined.into());
        }
    }

    fn get_default_biome_mut(&mut self) -> Result<&mut Biome> {
        return Ok(self.get_biome_mut(&self.get_default_biome_name()?)?);
    }

    pub fn update(
        &mut self,
        biome_args: Option<BiomeArg>,
        env: Option<Vec<Pair>>,
        alias: Option<Vec<Pair>>,
    ) -> Result<()> {
        let biome_to_update: &mut Biome = self.select_biome_mut(biome_args)?;
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
        main.constructors = Some(Commands {
            exec: vec![Command {
                exe: String::from("echo"),
                args: Some(vec![String::from("entering terrain")]),
            }],
        });
        main.destructors = Some(Commands {
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

#[cfg(test)]
mod test {
    use std::{borrow::BorrowMut, collections::HashMap, path::PathBuf};

    use anyhow::{anyhow, Result};

    use crate::types::{
        args::{BiomeArg, Pair},
        biomes::Biome,
        commands::{Command, Commands},
        errors::TerrainiumErrors,
    };

    use super::{parse_terrain, Terrain};

    #[test]
    fn parse_toml_full() -> Result<()> {
        let expected = test_data_terrain_full();
        let parsed = parse_terrain(&PathBuf::from("./example_configs/terrain.full.toml"))?;

        assert_eq!(expected, parsed);

        return Ok(());
    }

    #[test]
    fn parse_toml_without_biomes() -> Result<()> {
        let expected = test_data_terrain_without_biomes();
        let parsed = parse_terrain(&PathBuf::from(
            "./example_configs/terrain.without.biomes.toml",
        ))?;

        assert_eq!(expected, parsed);

        return Ok(());
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
        let terrain = test_data_terrain_full();
        let expected = &test_data_biome("example_biome2", "nano");

        let actual = terrain.get_biome(&"example_biome2".to_owned())?;

        assert_eq!(expected, actual);

        return Ok(());
    }

    #[test]
    fn get_biome_returns_error_if_not_present() -> Result<()> {
        let terrain = test_data_terrain_full();

        let non_existent = "non_existent".to_owned();
        let actual = terrain.get_biome(&non_existent).unwrap_err();

        assert_eq!(
            actual.to_string(),
            TerrainiumErrors::BiomeNotFound(non_existent).to_string()
        );

        return Ok(());
    }

    #[test]
    fn get_biome_returns_error_if_biomes_not_defined() -> Result<()> {
        let terrain = test_data_terrain_without_biomes();

        let biome = "example_biome".to_owned();
        let actual = terrain.get_biome(&biome).unwrap_err();

        assert_eq!(
            actual.to_string(),
            TerrainiumErrors::BiomesNotDefined.to_string()
        );

        return Ok(());
    }

    #[test]
    fn get_default_biome_name_returns_default_if_present() -> Result<()> {
        let terrain = test_data_terrain_full();

        let expected = "example_biome".to_owned();
        let actual = terrain.get_default_biome_name()?;

        assert_eq!(expected, actual);

        return Ok(());
    }

    #[test]
    fn get_default_biome_name_returns_error_if_no_biomes() -> Result<()> {
        let terrain = test_data_terrain_without_biomes();

        let expected = TerrainiumErrors::DefaultBiomeNotDefined.to_string();
        let actual = terrain.get_default_biome_name().unwrap_err().to_string();

        assert_eq!(expected, actual);

        return Ok(());
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
            exec: vec![
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("entering terrain")]),
                },
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("entering biome 'example_biome'")]),
                },
            ],
        };
        let destructor = Commands {
            exec: vec![
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("exiting terrain")]),
                },
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("exiting biome 'example_biome'")]),
                },
            ],
        };
        let expected = Biome {
            env: Some(env),
            alias: Some(alias),
            constructors: Some(constructor),
            destructors: Some(destructor),
        };

        let terrain = test_data_terrain_full();

        let actual = terrain.get_merged_biome(terrain.get_biome(&"example_biome".to_string())?);

        assert_eq!(expected, actual);

        return Ok(());
    }

    #[test]
    fn select_biome_returns_biome_if_biomearg_value() -> Result<()> {
        let terrain = test_data_terrain_full();
        let expected = &test_data_biome("example_biome2", "nano");

        let acutal = terrain.select_biome(Some(BiomeArg::Value("example_biome2".to_owned())))?;

        assert_eq!(expected, acutal);

        return Ok(());
    }

    #[test]
    fn select_biome_returns_default_if_biomearg_default() -> Result<()> {
        let terrain = test_data_terrain_full();
        let expected = &test_data_biome("example_biome", "nvim");

        let acutal = terrain.select_biome(Some(BiomeArg::Default))?;

        assert_eq!(expected, acutal);

        return Ok(());
    }

    #[test]
    fn select_biome_returns_main_terrain_if_biomearg_none() -> Result<()> {
        let terrain = test_data_terrain_full();
        let expected = &terrain.terrain;

        let acutal = terrain.select_biome(Some(BiomeArg::None))?;

        assert_eq!(expected, acutal);

        return Ok(());
    }

    #[test]
    fn select_biome_returns_default_if_no_biome_arg() -> Result<()> {
        let terrain = test_data_terrain_full();
        let expected = &test_data_biome("example_biome", "nvim");

        let acutal = terrain.select_biome(None)?;

        assert_eq!(expected, acutal);

        return Ok(());
    }

    #[test]
    fn select_biome_returns_main_terrain_if_no_biomearg_and_no_biomes_defined() -> Result<()> {
        let terrain = test_data_terrain_without_biomes();
        let expected = &terrain.terrain;

        let acutal = terrain.select_biome(None)?;

        assert_eq!(expected, acutal);

        return Ok(());
    }

    #[test]
    fn select_biome_returns_err_if_biome_not_present() -> Result<()> {
        let terrain = test_data_terrain_full();
        let expected = TerrainiumErrors::BiomeNotFound("non_existent".to_owned()).to_string();

        let acutal = terrain
            .select_biome(Some(BiomeArg::Value("non_existent".to_owned())))
            .unwrap_err()
            .to_string();

        assert_eq!(expected, acutal);

        return Ok(());
    }

    #[test]
    fn select_biome_returns_err_if_biomes_not_defined() -> Result<()> {
        let terrain = test_data_terrain_without_biomes();
        let expected = TerrainiumErrors::BiomesNotDefined.to_string();

        let acutal = terrain
            .select_biome(Some(BiomeArg::Value("non_existent".to_owned())))
            .unwrap_err()
            .to_string();

        assert_eq!(expected, acutal);

        return Ok(());
    }

    #[test]
    fn select_biome_returns_err_if_biomes_not_defined_and_biomeargs_default_passed() -> Result<()> {
        let terrain = test_data_terrain_without_biomes();
        let expected = TerrainiumErrors::DefaultBiomeNotDefined.to_string();

        let acutal = terrain
            .select_biome(Some(BiomeArg::Default))
            .unwrap_err()
            .to_string();

        assert_eq!(expected, acutal);

        return Ok(());
    }

    #[test]
    fn test_get_env() -> Result<()> {
        let terrain = test_data_terrain_full();

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

        return Ok(());
    }

    #[test]
    fn get_env_throws_err_if_no_env_defined() -> Result<()> {
        let terrain = Terrain {
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

        return Ok(());
    }

    #[test]
    fn get_alias_throws_err_if_no_aliases_defined() -> Result<()> {
        let terrain = Terrain {
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

        return Ok(());
    }

    #[test]
    fn test_get_alias() -> Result<()> {
        let terrain = test_data_terrain_full();

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

        return Ok(());
    }

    #[test]
    fn terrain_get_returns_main_terrain_when_biomearg_none_passed() -> Result<()> {
        let terrain = test_data_terrain_full();
        let expected = test_data_main_terrain_full();

        let actual = terrain.get(Some(BiomeArg::None))?;

        assert_eq!(expected, actual);

        return Ok(());
    }

    #[test]
    fn terrain_get_returns_merged_default_when_biomearg_default_passed() -> Result<()> {
        let terrain = test_data_terrain_full();
        let expected = test_data_merged_default();

        let actual = terrain.get(Some(BiomeArg::Default))?;

        assert_eq!(expected, actual);

        return Ok(());
    }

    #[test]
    fn terrain_get_returns_merged_when_biomearg_value_passed() -> Result<()> {
        let terrain = test_data_terrain_full();
        let expected = test_data_merged_example_biome2();

        let actual = terrain.get(Some(BiomeArg::Value("example_biome2".to_string())))?;

        assert_eq!(expected, actual);

        return Ok(());
    }

    #[test]
    fn terrain_get_returns_merged_default_when_biomearg_not_passed() -> Result<()> {
        let terrain = test_data_terrain_full();
        let expected = test_data_merged_default();

        let actual = terrain.get(None)?;

        assert_eq!(expected, actual);

        return Ok(());
    }

    #[test]
    fn terrain_get_returns_main_terrain_when_biomearg_not_passed_and_no_default() -> Result<()> {
        let terrain = test_data_terrain_without_biomes();
        let expected = test_data_terrain_without_biomes_main();

        let actual = terrain.get(None)?;

        assert_eq!(expected, actual);

        return Ok(());
    }

    #[test]
    fn terrain_get_returns_error_when_biomearg_default_passed_but_not_defined() -> Result<()> {
        let terrain = test_data_terrain_without_biomes();
        let expected = TerrainiumErrors::DefaultBiomeNotDefined.to_string();

        let actual = terrain
            .get(Some(BiomeArg::Default))
            .unwrap_err()
            .to_string();

        assert_eq!(expected, actual);

        return Ok(());
    }

    #[test]
    fn terrain_get_returns_error_when_biomearg_value_passed_but_not_defined() -> Result<()> {
        let terrain = test_data_terrain_full();
        let expected = TerrainiumErrors::BiomeNotFound("non_existent".to_string()).to_string();

        let actual = terrain
            .get(Some(BiomeArg::Value("non_existent".to_string())))
            .unwrap_err()
            .to_string();

        assert_eq!(expected, actual);

        return Ok(());
    }

    #[test]
    fn update_default_biome_works() -> Result<()> {
        let mut expected = test_data_terrain_full();
        expected.default_biome = Some("example_biome2".to_string());

        let mut terrain = test_data_terrain_full();
        terrain.update_default_biome("example_biome2".to_string())?;
        let actual = terrain;

        assert_eq!(expected, actual);

        return Ok(());
    }

    #[test]
    fn update_default_biome_returns_error_if_not_found() -> Result<()> {
        let expected = TerrainiumErrors::BiomeNotFound("non_existent".to_string()).to_string();

        let mut terrain = test_data_terrain_full();
        let actual = terrain
            .update_default_biome("non_existent".to_string())
            .unwrap_err()
            .to_string();

        assert_eq!(expected, actual);

        return Ok(());
    }

    #[test]
    fn add_biome_works() -> Result<()> {
        let mut expected = test_data_terrain_full();
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

        let mut terrain = test_data_terrain_full();
        terrain.add_biome(&"new_biome".to_string(), Biome::new())?;

        assert_eq!(expected, terrain);

        return Ok(());
    }

    #[test]
    fn add_biome_works_when_none_present() -> Result<()> {
        let mut expected = test_data_terrain_without_biomes();
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

        let mut terrain = test_data_terrain_without_biomes();
        terrain.add_biome(&"new_biome".to_string(), Biome::new())?;

        assert_eq!(expected, terrain);

        return Ok(());
    }

    #[test]
    fn add_biome_fails_if_already_present() -> Result<()> {
        let expected =
            TerrainiumErrors::BiomeAlreadyExists("example_biome2".to_string()).to_string();

        let mut terrain = test_data_terrain_full();
        let actual = terrain
            .add_biome(&"example_biome2".to_string(), Biome::new())
            .unwrap_err()
            .to_string();

        assert_eq!(expected, actual);

        return Ok(());
    }

    #[test]
    fn get_biome_mut_returns_biome_if_present() -> Result<()> {
        let mut terrain = test_data_terrain_full();
        let expected = &mut test_data_biome("example_biome2", "nano");

        let actual = terrain.get_biome_mut(&"example_biome2".to_owned())?;

        assert_eq!(expected, actual);

        return Ok(());
    }

    #[test]
    fn get_biome_mut_returns_error_if_not_present() -> Result<()> {
        let mut terrain = test_data_terrain_full();

        let non_existent = "non_existent".to_owned();
        let actual = terrain.get_biome_mut(&non_existent).unwrap_err();

        assert_eq!(
            actual.to_string(),
            TerrainiumErrors::BiomeNotFound(non_existent).to_string()
        );

        return Ok(());
    }

    #[test]
    fn get_biome_mut_returns_error_if_biomes_not_defined() -> Result<()> {
        let mut terrain = test_data_terrain_without_biomes();

        let biome = "example_biome".to_owned();
        let actual = terrain.get_biome_mut(&biome).unwrap_err();

        assert_eq!(
            actual.to_string(),
            TerrainiumErrors::BiomesNotDefined.to_string()
        );

        return Ok(());
    }

    #[test]
    fn get_default_biome_mut_name_returns_default_if_present() -> Result<()> {
        let mut terrain = test_data_terrain_full();

        if let Some(biomes) = test_data_terrain_full().biomes.borrow_mut() {
            let expected = biomes.get_mut("example_biome").expect("to be present");
            let actual = terrain.get_default_biome_mut()?;

            assert_eq!(expected, actual);
        }

        return Ok(());
    }

    #[test]
    fn get_default_biome_mut_name_returns_error_if_no_biomes() -> Result<()> {
        let mut terrain = test_data_terrain_without_biomes();

        let expected = TerrainiumErrors::DefaultBiomeNotDefined.to_string();
        let actual = terrain.get_default_biome_mut().unwrap_err().to_string();

        assert_eq!(expected, actual);

        return Ok(());
    }

    #[test]
    fn update_works_full() -> Result<()> {
        let mut terrain = test_data_terrain_full();
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

        let mut expected = test_data_terrain_full();
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

        return Ok(());
    }

    #[test]
    fn update_works_only_env() -> Result<()> {
        let mut terrain = test_data_terrain_full();
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
        let mut expected = test_data_terrain_full();
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

        return Ok(());
    }

    #[test]
    fn update_works_only_alias() -> Result<()> {
        let mut terrain = test_data_terrain_full();
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

        let mut expected = test_data_terrain_full();
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

        return Ok(());
    }

    #[test]
    fn update_works_main_terrain() -> Result<()> {
        let mut terrain = test_data_terrain_full();
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

        let mut expected = test_data_terrain_full();
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

        return Ok(());
    }

    #[test]
    fn update_updates_nothing_if_nothing_passed() -> Result<()> {
        let mut terrain = test_data_terrain_full();
        let biome_args = BiomeArg::None;
        let expected = test_data_terrain_full();
        terrain.update(Some(biome_args), None, None)?;

        assert_eq!(expected, terrain);

        return Ok(());
    }
    fn test_data_biome(name: &str, editor: &str) -> Biome {
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
            exec: vec![Command {
                exe: String::from("echo"),
                args: Some(vec![String::from("entering biome '") + &name + "'"]),
            }],
        };
        let destructor = Commands {
            exec: vec![Command {
                exe: String::from("echo"),
                args: Some(vec![String::from("exiting biome '") + &name + "'"]),
            }],
        };

        return Biome {
            env: Some(env),
            alias: Some(alias),
            constructors: Some(constructor),
            destructors: Some(destructor),
        };
    }

    fn test_data_main_terrain_full() -> Biome {
        return test_data_terrain_full().terrain;
    }

    fn test_data_merged_default() -> Biome {
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
            exec: vec![
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("entering terrain")]),
                },
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("entering biome 'example_biome'")]),
                },
            ],
        };
        let destructor = Commands {
            exec: vec![
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("exiting terrain")]),
                },
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("exiting biome 'example_biome'")]),
                },
            ],
        };
        return Biome {
            env: Some(env),
            alias: Some(alias),
            constructors: Some(constructor),
            destructors: Some(destructor),
        };
    }

    fn test_data_merged_example_biome2() -> Biome {
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
            exec: vec![
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("entering terrain")]),
                },
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("entering biome 'example_biome2'")]),
                },
            ],
        };
        let destructor = Commands {
            exec: vec![
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("exiting terrain")]),
                },
                Command {
                    exe: String::from("echo"),
                    args: Some(vec![String::from("exiting biome 'example_biome2'")]),
                },
            ],
        };
        return Biome {
            env: Some(env),
            alias: Some(alias),
            constructors: Some(constructor),
            destructors: Some(destructor),
        };
    }

    fn test_data_terrain_full() -> Terrain {
        let mut env = HashMap::<String, String>::new();
        env.insert(String::from("EDITOR"), String::from("vim"));
        env.insert(String::from("TEST"), String::from("value"));
        let mut alias = HashMap::<String, String>::new();
        alias.insert(String::from("tedit"), String::from("terrainium edit"));
        alias.insert(String::from("tenter"), String::from("terrainium enter"));
        let constructor = Commands {
            exec: vec![Command {
                exe: String::from("echo"),
                args: Some(vec![String::from("entering terrain")]),
            }],
        };
        let destructor = Commands {
            exec: vec![Command {
                exe: String::from("echo"),
                args: Some(vec![String::from("exiting terrain")]),
            }],
        };

        let biome1_name = "example_biome";
        let biome1 = test_data_biome(biome1_name, "nvim");
        let biome2_name = "example_biome2";
        let biome2 = test_data_biome(biome2_name, "nano");
        let mut biomes = HashMap::<String, Biome>::new();
        biomes.insert(biome1_name.to_owned(), biome1);
        biomes.insert(biome2_name.to_owned(), biome2);

        return Terrain {
            terrain: Biome {
                env: Some(env),
                alias: Some(alias),
                constructors: Some(constructor),
                destructors: Some(destructor),
            },
            default_biome: Some("example_biome".to_owned()),
            biomes: Some(biomes),
        };
    }

    fn test_data_terrain_without_biomes_main() -> Biome {
        return test_data_terrain_without_biomes().terrain;
    }

    fn test_data_terrain_without_biomes() -> Terrain {
        let mut env = HashMap::<String, String>::new();
        env.insert(String::from("VAR1"), String::from("val1"));
        env.insert(String::from("VAR2"), String::from("val2"));
        env.insert(String::from("VAR3"), String::from("val3"));
        let mut alias = HashMap::<String, String>::new();
        alias.insert(String::from("alias1"), String::from("run1"));
        alias.insert(String::from("alias2"), String::from("run2"));
        alias.insert(String::from("alias3"), String::from("run3"));
        let constructor = Commands {
            exec: vec![Command {
                exe: String::from("run1"),
                args: Some(vec![String::from("something-new")]),
            }],
        };
        let destructor = Commands {
            exec: vec![Command {
                exe: String::from("stop1"),
                args: Some(vec![String::from("something-old")]),
            }],
        };
        return Terrain {
            terrain: Biome {
                env: Some(env),
                alias: Some(alias),
                constructors: Some(constructor),
                destructors: Some(destructor),
            },
            default_biome: None,
            biomes: None,
        };
    }
}
