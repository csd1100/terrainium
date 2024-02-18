use std::collections::HashMap;

use anyhow::{Ok, Result};
use serde::{Deserialize, Serialize};

use crate::helpers::operations::{find_in_hashmaps, get_merged_hashmaps};

use super::{
    commands::{get_merged_commands, Command, Commands},
    errors::TerrainiumErrors,
};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Biome {
    pub env: Option<HashMap<String, String>>,
    pub alias: Option<HashMap<String, String>>,
    pub constructors: Option<Commands>,
    pub destructors: Option<Commands>,
}

impl Biome {
    pub fn new() -> Biome {
        Biome {
            env: Some(HashMap::<String, String>::new()),
            alias: Some(HashMap::<String, String>::new()),
            constructors: None,
            destructors: None,
        }
    }

    pub fn update_env(&mut self, k: String, v: String) {
        if self.env.is_none() {
            self.env = Some(HashMap::<String, String>::new());
        }

        if let Some(env) = self.env.as_mut() {
            env.insert(k, v);
        }
    }

    pub fn update_alias(&mut self, k: String, v: String) {
        if self.alias.is_none() {
            self.alias = Some(HashMap::<String, String>::new());
        }

        if let Some(alias) = self.alias.as_mut() {
            alias.insert(k, v);
        }
    }

    pub fn find_envs(&self, tofind: Vec<String>) -> Result<HashMap<String, Option<String>>> {
        match find_in_hashmaps(&self.env, tofind) {
            Result::Ok(envs) => Ok(envs),
            Err(_) => Err(TerrainiumErrors::EnvsNotDefined.into()),
        }
    }

    pub fn find_aliases(&self, tofind: Vec<String>) -> Result<HashMap<String, Option<String>>> {
        match find_in_hashmaps(&self.alias, tofind) {
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

    fn merge_env(&self, other: &Self) -> Option<HashMap<String, String>> {
        get_merged_hashmaps(&self.env, &other.env)
    }

    fn merge_alias(&self, other: &Self) -> Option<HashMap<String, String>> {
        get_merged_hashmaps(&self.alias, &other.alias)
    }

    fn merge_constructors(&self, other: &Self) -> Option<Commands> {
        get_merged_commands(&self.constructors, &other.constructors)
    }

    fn merge_destructors(&self, other: &Self) -> Option<Commands> {
        get_merged_commands(&self.destructors, &other.destructors)
    }
}

impl Default for Biome {
    fn default() -> Self {
        let name = String::from("example_biome");
        let mut env = HashMap::<String, String>::new();
        env.insert(String::from("EDITOR"), String::from("vim"));
        let mut alias = HashMap::<String, String>::new();
        alias.insert(String::from("tedit"), String::from("terrainium edit"));
        alias.insert(String::from("tenter"), String::from("terrainium enter"));
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BiomeWithName {
    pub name: String,
    pub biome: Biome,
}
