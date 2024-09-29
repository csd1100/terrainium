use crate::client::types::biome::Biome;
use crate::client::types::commands::Commands;
use crate::common::types::command::Command;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Terrain {
    terrain: Biome,
    biomes: BTreeMap<String, Biome>,
    default_biome: Option<String>,
}

impl Terrain {}

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

    pub fn default_biome(&self) -> &Option<String> {
        &self.default_biome
    }

    pub fn terrain(&self) -> &Biome {
        &self.terrain
    }

    pub fn biomes(&self) -> &BTreeMap<String, Biome> {
        &self.biomes
    }

    pub fn merged(&self, selected_biome: &Option<String>) -> Result<Biome> {
        let selected = self.select_biome(selected_biome.clone())?.1;
        if selected == &self.terrain {
            Ok(selected.clone())
        } else {
            Ok(self.terrain.merge(selected))
        }
    }

    pub fn merged_aliases(
        &self,
        selected_biome: &Option<String>,
    ) -> Result<BTreeMap<String, String>> {
        let selected = self.select_biome(selected_biome.clone())?.1;
        if selected == &self.terrain {
            Ok(selected.aliases().clone())
        } else {
            Ok(self.terrain.append_aliases(selected))
        }
    }

    pub fn merged_envs(&self, selected_biome: &Option<String>) -> Result<BTreeMap<String, String>> {
        let selected = self.select_biome(selected_biome.clone())?.1;
        if selected == &self.terrain {
            Ok(selected.envs().clone())
        } else {
            Ok(self.terrain.append_envs(selected))
        }
    }

    pub fn merged_constructors(&self, selected_biome: &Option<String>) -> Result<Commands> {
        let selected = self.select_biome(selected_biome.clone())?.1;
        if selected == &self.terrain {
            Ok(selected.constructors().clone())
        } else {
            Ok(self.terrain.append_constructors(selected))
        }
    }

    pub fn merged_destructors(&self, selected_biome: &Option<String>) -> Result<Commands> {
        let selected = self.select_biome(selected_biome.clone())?.1;
        if selected == &self.terrain {
            Ok(selected.destructors().clone())
        } else {
            Ok(self.terrain.append_destructors(selected))
        }
    }

    pub(crate) fn select_biome(&self, selected: Option<String>) -> Result<(String, &Biome)> {
        let selected = match selected {
            None => self.default_biome.clone(),
            Some(selected) => Some(selected),
        };
        match selected {
            None => Ok(("none".to_string(), &self.terrain)),
            Some(selected) => {
                if selected == "none" {
                    Ok(("none".to_string(), &self.terrain))
                } else if let Some(biome) = self.biomes.get(&selected) {
                    Ok((selected, biome))
                } else {
                    Err(anyhow!("the biome {:?} does not exists", selected))
                }
            }
        }
    }

    pub fn from_toml(toml_str: String) -> Result<Self> {
        toml::from_str(&toml_str).context("failed to parse terrain from toml")
    }

    pub fn to_toml(&self) -> Result<String> {
        toml::to_string(&self).context("failed to convert terrain to toml")
    }

    pub(crate) fn set_default(&mut self, new_default: String) {
        self.default_biome = Some(new_default);
    }

    pub(crate) fn update(&mut self, biome_name: String, updated: Biome) {
        if biome_name == "none" {
            self.terrain = updated
        } else {
            self.biomes.insert(biome_name, updated);
        }
    }

    pub fn example() -> Self {
        let terrain = Biome::example();

        let mut envs: BTreeMap<String, String> = BTreeMap::new();
        envs.insert("EDITOR".to_string(), "nvim".to_string());

        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        aliases.insert(
            "tenter".to_string(),
            "terrainium enter --biome example_biome".to_string(),
        );

        let constructors: Commands = Commands::new(
            vec![Command::new(
                "/bin/echo".to_string(),
                vec!["entering biome example_biome".to_string()],
            )],
            vec![Command::new(
                "/bin/bash".to_string(),
                vec![
                    "-c".to_string(),
                    "$PWD/tests/scripts/print_num_for_10_sec".to_string(),
                ],
            )],
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

#[cfg(test)]
pub mod test {
    use crate::client::types::biome::Biome;
    use crate::client::types::commands::Commands;
    use crate::client::types::terrain::Terrain;
    use crate::common::types::command::Command;
    use std::collections::BTreeMap;

    pub fn force_set_invalid_default_biome(terrain: &mut Terrain, default_biome: Option<String>) {
        terrain.default_biome = default_biome;
    }

    pub fn add_biome(terrain: &mut Terrain, name: String, biome: Biome) {
        terrain.biomes.insert(name, biome);
    }

    pub fn get_test_biome(name: String, editor: String) -> Biome {
        let mut biome_envs: BTreeMap<String, String> = BTreeMap::new();
        biome_envs.insert("EDITOR".to_string(), editor);

        let mut biome_aliases: BTreeMap<String, String> = BTreeMap::new();
        biome_aliases.insert(
            "tenter".to_string(),
            "terrainium enter --biome ".to_string() + &name,
        );
        let biome_constructor_foreground: Vec<Command> = vec![Command::new(
            "/bin/echo".to_string(),
            vec!["entering biome ".to_string() + &name],
        )];
        let biome_constructor_background: Vec<Command> = vec![];
        let biome_destructor_foreground: Vec<Command> = vec![Command::new(
            "/bin/echo".to_string(),
            vec!["exiting biome ".to_string() + &name],
        )];
        let biome_destructor_background: Vec<Command> = vec![];

        let biome_constructor =
            Commands::new(biome_constructor_foreground, biome_constructor_background);
        let biome_destructor =
            Commands::new(biome_destructor_foreground, biome_destructor_background);
        Biome::new(
            biome_envs,
            biome_aliases,
            biome_constructor,
            biome_destructor,
        )
    }
}
