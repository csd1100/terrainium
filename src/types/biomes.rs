use serde::{Deserialize, Serialize};

use super::{aliases::KeyValue, commands::Command, commands::Commands};

#[derive(Serialize, Deserialize, Debug)]
pub struct Biome {
    pub name: String,
    pub env: Option<KeyValue>,
    pub aliases: Option<KeyValue>,
    pub construct: Option<Commands>,
    pub destruct: Option<Commands>,
}

impl Biome {
    pub fn new() -> Biome {
        return Biome {
            name: String::new(),
            env: Some(KeyValue::new()),
            aliases: Some(KeyValue::new()),
            construct: None,
            destruct: None,
        };
    }
}

impl Default for Biome {
    fn default() -> Self {
        let name = String::from("biome1");
        let mut env = KeyValue::new();
        env.insert(String::from("EDITOR"), String::from("nvim"));
        let mut aliases = KeyValue::new();
        aliases.insert(
            String::from("tenter"),
            String::from("terrain enter -b ") + &name,
        );
        let construct = Commands {
            exec: vec![Command {
                exe: String::from("echo"),
                args: Some(vec!["entering biome ".to_string() + &name]),
            }],
        };
        let destruct = Commands {
            exec: vec![Command {
                exe: String::from("echo"),
                args: Some(vec!["exiting biome ".to_string() + &name]),
            }],
        };

        return Biome {
            name,
            env: Some(env),
            aliases: Some(aliases),
            construct: Some(construct),
            destruct: Some(destruct),
        };
    }
}
