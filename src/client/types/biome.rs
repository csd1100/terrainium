use crate::client::types::command::Command;
use crate::client::types::commands::Commands;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;

#[cfg_attr(feature = "terrain-schema", derive(JsonSchema))]
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

    pub fn aliases(&self) -> &BTreeMap<String, String> {
        &self.aliases
    }

    pub(crate) fn envs(&self) -> &BTreeMap<String, String> {
        &self.envs
    }

    pub(crate) fn constructors(&self) -> &Commands {
        &self.constructors
    }

    pub(crate) fn destructors(&self) -> &Commands {
        &self.destructors
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

    pub(crate) fn set_envs(&mut self, envs: BTreeMap<String, String>) {
        self.envs = envs;
    }

    pub(crate) fn set_aliases(&mut self, aliases: BTreeMap<String, String>) {
        self.aliases = aliases;
    }

    fn recursive_substitute_envs(&self, env_to_substitute: String) -> String {
        if env_to_substitute.starts_with("$") {
            let env = env_to_substitute.strip_prefix("$").unwrap();
            if self.envs.contains_key(env) || std::env::var(env).is_ok() {
                let recurse = if let Some(val) = self.envs.get(env) {
                    val.to_string()
                } else if let Ok(val) = std::env::var(env) {
                    val
                } else {
                    env_to_substitute
                };
                return self.recursive_substitute_envs(recurse);
            }
        }
        env_to_substitute
    }

    pub(crate) fn substitute_envs(&mut self) {
        let biome_envs = self.envs();
        let substituted_envs: Vec<(String, String)> = biome_envs
            .iter()
            .map(|(key, value)| (key.clone(), self.recursive_substitute_envs(value.clone())))
            .collect();

        self.set_envs(BTreeMap::from_iter(substituted_envs));
    }

    pub fn example() -> Self {
        let mut envs: BTreeMap<String, String> = BTreeMap::new();
        envs.insert("EDITOR".to_string(), "vim".to_string());
        envs.insert("NESTED_POINTER".to_string(), "$POINTER".to_string());
        envs.insert("NULL_POINTER".to_string(), "$NULL".to_string());
        envs.insert("PAGER".to_string(), "less".to_string());
        envs.insert("POINTER".to_string(), "$REAL".to_string());
        envs.insert("REAL".to_string(), "real_value".to_string());

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

    #[cfg(test)]
    pub(crate) fn add_env(&mut self, env: (String, String)) {
        self.envs.insert(env.0, env.1);
    }
}
