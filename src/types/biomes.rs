use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::commands::{Command, Commands};

#[derive(Debug, Serialize, Deserialize)]
pub struct Biome {
    pub env: Option<HashMap<String, String>>,
    pub alias: Option<HashMap<String, String>>,
    pub constructor: Option<Commands>,
    pub destructor: Option<Commands>,
}

impl Biome {
    pub fn new() -> Biome {
        return Biome {
            env: Some(HashMap::<String, String>::new()),
            alias: Some(HashMap::<String, String>::new()),
            constructor: None,
            destructor: None,
        };
    }
}

impl Biome {
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
        Self {
            env: Some(env),
            alias: Some(alias),
            constructor: Some(constructor),
            destructor: Some(destructor),
        }
    }
}
