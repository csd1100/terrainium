use crate::client::types::commands::Commands;
use crate::common::types::command::Command;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
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
        constructors: Commands,
        destructors: Commands,
    ) -> Self {
        Biome {
            envs,
            aliases,
            constructors,
            destructors,
        }
    }

    pub fn aliases(&self) -> BTreeMap<String, String> {
        self.aliases.clone()
    }

    pub(crate) fn envs(&self) -> BTreeMap<String, String> {
        self.envs.clone()
    }

    pub(crate) fn constructors(&self) -> Commands {
        self.constructors.clone()
    }

    pub(crate) fn destructors(&self) -> Commands {
        self.destructors.clone()
    }

    pub fn merge(&self, another: &Biome) -> Biome {
        Biome::new(
            self.append_envs(another),
            self.append_aliases(another),
            self.append_constructors(another),
            self.append_destructors(another),
        )
    }

    pub(crate) fn append_destructors(&self, another: &Biome) -> Commands {
        let mut destructors = self.destructors.clone();
        let mut another_destructors = another.destructors.clone();

        destructors.append(&mut another_destructors);
        destructors
    }

    pub(crate) fn append_constructors(&self, another: &Biome) -> Commands {
        let mut constructors = self.constructors.clone();
        let mut another_constructors = another.constructors.clone();

        constructors.append(&mut another_constructors);
        constructors
    }

    pub(crate) fn append_aliases(&self, another: &Biome) -> BTreeMap<String, String> {
        let mut aliases = self.aliases.clone();
        let mut another_aliases = another.aliases.clone();

        aliases.append(&mut another_aliases);
        aliases
    }

    pub(crate) fn append_envs(&self, another: &Biome) -> BTreeMap<String, String> {
        let mut envs = self.envs.clone();
        let mut another_envs = another.envs.clone();

        envs.append(&mut another_envs);
        envs
    }

    pub fn example() -> Self {
        let mut envs: BTreeMap<String, String> = BTreeMap::new();
        envs.insert("EDITOR".to_string(), "vim".to_string());
        envs.insert("PAGER".to_string(), "less".to_string());

        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        aliases.insert("tenter".to_string(), "terrainium enter".to_string());
        aliases.insert("texit".to_string(), "terrainium exit".to_string());

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
