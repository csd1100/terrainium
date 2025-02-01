use crate::client::types::biome::Biome;
use crate::client::types::command::Command;
use crate::client::types::commands::Commands;
use anyhow::{anyhow, Context, Result};
#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[cfg_attr(feature = "terrain-schema", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Clone)]
pub struct Terrain {
    #[serde(default = "schema_url", rename(serialize = "$schema"))]
    schema: String,

    name: String,
    auto_apply: AutoApply,
    terrain: Biome,
    biomes: BTreeMap<String, Biome>,
    default_biome: Option<String>,
}

#[cfg_attr(feature = "terrain-schema", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct AutoApply {
    enabled: bool,
    background: bool,
    replace: bool,
}

impl AutoApply {
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            background: false,
            replace: false,
        }
    }

    pub fn background() -> Self {
        Self {
            enabled: true,
            background: true,
            replace: false,
        }
    }

    pub fn replace() -> Self {
        Self {
            enabled: true,
            background: false,
            replace: true,
        }
    }

    pub fn all() -> Self {
        Self {
            enabled: true,
            background: true,
            replace: true,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled && !self.background && !self.replace
    }

    pub fn is_background(&self) -> bool {
        self.enabled && self.background
    }

    pub fn is_replace(&self) -> bool {
        self.enabled && self.replace
    }

    pub fn is_all(&self) -> bool {
        self.enabled && self.background && self.replace
    }
}

impl From<AutoApply> for String {
    fn from(value: AutoApply) -> Self {
        if value.is_all() {
            "all"
        } else if value.is_enabled() {
            "enabled"
        } else if value.is_replace() {
            "replaced"
        } else if value.is_background() {
            "background"
        } else {
            "off"
        }
        .to_string()
    }
}

pub fn schema_url() -> String {
    "https://raw.githubusercontent.com/csd1100/terrainium/main/schema/terrain-schema.json"
        .to_string()
}

impl Terrain {
    pub fn new(
        terrain: Biome,
        biomes: BTreeMap<String, Biome>,
        default_biome: Option<String>,
        auto_apply: AutoApply,
    ) -> Self {
        let name = std::env::current_dir()
            .expect("failed to get current directory")
            .file_name()
            .expect("failed to get current directory name")
            .to_str()
            .expect("failed to convert directory name to string")
            .to_string();

        Terrain {
            name,
            schema: schema_url(),
            auto_apply,
            terrain,
            biomes,
            default_biome,
        }
    }

    pub fn default_biome(&self) -> &Option<String> {
        &self.default_biome
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn terrain(&self) -> &Biome {
        &self.terrain
    }

    pub fn terrain_mut(&mut self) -> &mut Biome {
        &mut self.terrain
    }

    pub fn biomes(&self) -> &BTreeMap<String, Biome> {
        &self.biomes
    }

    pub fn auto_apply(&self) -> &AutoApply {
        &self.auto_apply
    }

    pub fn set_auto_apply(&mut self, auto_apply: AutoApply) {
        self.auto_apply = auto_apply;
    }

    pub fn merged(&self, selected_biome: &Option<String>) -> Result<Biome> {
        let selected = self.select_biome(selected_biome)?.1;
        if selected == &self.terrain {
            Ok(self.terrain.clone())
        } else {
            Ok(self.terrain.merge(selected))
        }
    }

    pub(crate) fn select_biome(&self, selected: &Option<String>) -> Result<(String, &Biome)> {
        let selected = match selected {
            None => self.default_biome.clone(),
            Some(selected) => Some(selected.clone()),
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

        let mut biome_envs: BTreeMap<String, String> = BTreeMap::new();
        biome_envs.insert("EDITOR".to_string(), "nvim".to_string());
        biome_envs.insert("ENV_VAR".to_string(), "overridden_env_val".to_string());

        let mut biome_aliases: BTreeMap<String, String> = BTreeMap::new();
        biome_aliases.insert(
            "tenter".to_string(),
            "terrainium enter --biome example_biome".to_string(),
        );

        let biome_constructors: Commands = Commands::new(
            vec![Command::new(
                "/bin/echo".to_string(),
                vec!["entering biome example_biome".to_string()],
                None,
            )],
            vec![
                Command::new(
                    "/bin/bash".to_string(),
                    vec!["-c".to_string(), "./print_num_for_10_sec".to_string()],
                    Some(PathBuf::from("../tests/scripts")),
                ),
                Command::new(
                    "/bin/bash".to_string(),
                    vec!["-c".to_string(), "./print_num_for_10_sec".to_string()],
                    Some(PathBuf::from("/home/user/work/scripts")),
                ),
            ],
        );

        let biome_destructors: Commands = Commands::new(
            vec![Command::new(
                "/bin/echo".to_string(),
                vec!["exiting biome example_biome".to_string()],
                None,
            )],
            vec![Command::new(
                "/bin/bash".to_string(),
                vec!["-c".to_string(), "./print_num_for_10_sec".to_string()],
                Some(PathBuf::from("./tests/scripts")),
            )],
        );

        let example_biome = Biome::new(
            biome_envs,
            biome_aliases,
            biome_constructors,
            biome_destructors,
        );

        let mut biomes: BTreeMap<String, Biome> = BTreeMap::new();
        biomes.insert(String::from("example_biome"), example_biome);

        Terrain::new(
            terrain,
            biomes,
            Some(String::from("example_biome")),
            AutoApply {
                enabled: false,
                background: false,
                replace: false,
            },
        )
    }
}

impl Default for Terrain {
    fn default() -> Self {
        Terrain::new(
            Biome::default(),
            BTreeMap::new(),
            None,
            AutoApply {
                enabled: false,
                background: false,
                replace: false,
            },
        )
    }
}

#[cfg(test)]
pub mod tests {
    use crate::client::types::biome::Biome;
    use crate::client::types::command::Command;
    use crate::client::types::commands::Commands;
    use crate::client::types::terrain::Terrain;
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
            None,
        )];
        let biome_constructor_background: Vec<Command> = vec![];
        let biome_destructor_foreground: Vec<Command> = vec![Command::new(
            "/bin/echo".to_string(),
            vec!["exiting biome ".to_string() + &name],
            None,
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
