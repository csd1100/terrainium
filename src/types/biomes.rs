use std::collections::BTreeMap;

use anyhow::{Ok, Result};
#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::helpers::operations::{find_in_maps, get_merged_maps};

use super::{
    commands::{get_merged_commands, Command, Commands},
    errors::TerrainiumErrors,
};

#[cfg_attr(feature = "terrain-schema", derive(JsonSchema))]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Biome {
    pub env: Option<BTreeMap<String, String>>,
    pub alias: Option<BTreeMap<String, String>>,
    pub constructors: Option<Commands>,
    pub destructors: Option<Commands>,
}

impl Biome {
    pub fn new() -> Biome {
        Biome {
            env: Some(BTreeMap::<String, String>::new()),
            alias: Some(BTreeMap::<String, String>::new()),
            constructors: None,
            destructors: None,
        }
    }

    pub fn get_constructors(self) -> Option<Commands> {
        self.constructors
    }

    pub fn get_detructors(self) -> Option<Commands> {
        self.destructors
    }

    pub fn update_env(&mut self, k: String, v: String) {
        if self.env.is_none() {
            self.env = Some(BTreeMap::<String, String>::new());
        }

        if let Some(env) = self.env.as_mut() {
            env.insert(k, v);
        }
    }

    pub fn update_alias(&mut self, k: String, v: String) {
        if self.alias.is_none() {
            self.alias = Some(BTreeMap::<String, String>::new());
        }

        if let Some(alias) = self.alias.as_mut() {
            alias.insert(k, v);
        }
    }

    pub fn find_envs(&self, tofind: Vec<String>) -> Result<BTreeMap<String, Option<String>>> {
        match find_in_maps(&self.env, tofind) {
            Result::Ok(envs) => Ok(envs),
            Err(_) => Err(TerrainiumErrors::EnvsNotDefined.into()),
        }
    }

    pub fn find_aliases(&self, tofind: Vec<String>) -> Result<BTreeMap<String, Option<String>>> {
        match find_in_maps(&self.alias, tofind) {
            Result::Ok(aliases) => Ok(aliases),
            Err(_err) => Err(TerrainiumErrors::AliasesNotDefined.into()),
        }
    }

    pub fn merge(&self, other: &Self) -> Self {
        let mut env = None;
        let mut alias = None;
        let mut constructors = None;
        let mut destructors = None;

        if self.env.is_some() || other.env.is_some() {
            env = self.merge_env(other);
        }

        if self.alias.is_some() || other.alias.is_some() {
            alias = self.merge_alias(other);
        }

        if self.constructors.is_some() || other.constructors.is_some() {
            constructors = self.merge_constructors(other);
        }

        if self.destructors.is_some() || other.destructors.is_some() {
            destructors = self.merge_destructors(other);
        }

        Biome {
            env,
            alias,
            constructors,
            destructors,
        }
    }

    fn merge_env(&self, other: &Self) -> Option<BTreeMap<String, String>> {
        get_merged_maps(&self.env, &other.env)
    }

    fn merge_alias(&self, other: &Self) -> Option<BTreeMap<String, String>> {
        get_merged_maps(&self.alias, &other.alias)
    }

    fn merge_constructors(&self, other: &Self) -> Option<Commands> {
        get_merged_commands(&self.constructors, &other.constructors)
    }

    fn merge_destructors(&self, other: &Self) -> Option<Commands> {
        get_merged_commands(&self.destructors, &other.destructors)
    }

    pub fn example() -> Self {
        let name = String::from("example_biome");
        let mut env = BTreeMap::<String, String>::new();
        env.insert(String::from("EDITOR"), String::from("nvim"));
        let mut alias = BTreeMap::<String, String>::new();
        alias.insert(String::from("tenter"), String::from("terrainium enter --biome example_biome"));
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
        Self {
            env: Some(env),
            alias: Some(alias),
            constructors: Some(constructor),
            destructors: Some(destructor),
        }
    }
}

impl Default for Biome {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BiomeWithName {
    pub name: String,
    pub biome: Biome,
}
