use crate::common::types::command::Command;
use crate::common::types::commands::Commands;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Default)]
pub struct Biome {
    envs: BTreeMap<String, String>,
    aliases: BTreeMap<String, String>,
    constructors: Commands,
    destructors: Commands,
}

impl Biome {
    pub fn new(
        envs: BTreeMap<String, String>,
        aliases: BTreeMap<String, String>,
        foreground: Commands,
        background: Commands,
    ) -> Self {
        Biome {
            envs,
            aliases,
            constructors: foreground,
            destructors: background,
        }
    }

    pub fn example() -> Self {
        let mut envs: BTreeMap<String, String> = BTreeMap::new();
        envs.insert("EDITOR".to_string(), "vim".to_string());

        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        aliases.insert("tenter".to_string(), "terrainium enter".to_string());

        let constructors: Commands = Commands::new(
            vec![Command::new(
                "/bin/echo".to_string(),
                vec!["entering terrain".to_string()],
            )],
            vec![],
        );

        let destructors: Commands = Commands::new(
            vec![Command::new(
                "/bin/echo".to_string(),
                vec!["exiting terrain".to_string()],
            )],
            vec![],
        );
        Biome::new(envs, aliases, constructors, destructors)
    }
}
