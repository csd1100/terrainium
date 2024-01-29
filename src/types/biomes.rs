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
