use crate::client::types::command::Command;
use crate::client::types::commands::Commands;
use regex::Regex;
#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

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

    fn get_envs_to_substitute(str_to_parse: &str) -> Vec<String> {
        let mut result = vec![];
        let re = Regex::new(r"\$\{(.*?)}").expect("Regex parse failed");
        for (_, [val]) in re.captures_iter(str_to_parse).map(|c| c.extract()) {
            result.push(val.to_string());
        }
        result
    }

    fn recursive_substitute_envs(
        &self,
        result_string: String,
        envs_to_substitute: Vec<String>,
    ) -> String {
        let mut envs_to_substitute = envs_to_substitute;
        let mut result_string = result_string;

        // recurse till envs_to_substitute is not empty
        if !envs_to_substitute.is_empty() {
            let env = envs_to_substitute.pop().unwrap();

            if self.envs.contains_key(&env) || std::env::var(&env).is_ok() {
                let env_val = if let Some(env_val) = self.envs.get(&env) {
                    env_val.to_string()
                } else {
                    std::env::var(&env).unwrap()
                };

                // if value present in terrain envs or system envs replace the value
                let value_to_replace = format!("${{{}}}", &env);
                result_string = result_string.replace(&value_to_replace, &env_val);

                // if the new value is also env ref add that to substitute list
                let new_env_to_substitute = Self::get_envs_to_substitute(&env_val);
                envs_to_substitute.extend(new_env_to_substitute);
            }
            return self.recursive_substitute_envs(result_string, envs_to_substitute);
        }
        result_string
    }

    pub(crate) fn substitute_envs(&mut self) {
        let biome_envs = self.envs();
        let substituted_envs: Vec<(String, String)> = biome_envs
            .iter()
            .map(|(key, value)| {
                let envs_to_substitute = Self::get_envs_to_substitute(value);
                (
                    key.clone(),
                    self.recursive_substitute_envs(value.clone(), envs_to_substitute),
                )
            })
            .collect();

        self.set_envs(BTreeMap::from_iter(substituted_envs));
    }

    pub(crate) fn substitute_cwd(&mut self, terrain_dir: &Path) {
        self.constructors.substitute_cwd(terrain_dir);
        self.destructors.substitute_cwd(terrain_dir);
    }

    pub fn example() -> Self {
        let mut envs: BTreeMap<String, String> = BTreeMap::new();
        envs.insert("EDITOR".to_string(), "vim".to_string());
        envs.insert("ENV_VAR".to_string(), "env_val".to_string());
        envs.insert(
            "NESTED_POINTER".to_string(),
            "${POINTER_ENV_VAR}-${ENV_VAR}-${NULL_POINTER}".to_string(),
        );
        envs.insert("NULL_POINTER".to_string(), "${NULL}".to_string());
        envs.insert("PAGER".to_string(), "less".to_string());
        envs.insert("POINTER_ENV_VAR".to_string(), "${ENV_VAR}".to_string());

        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        aliases.insert("tenter".to_string(), "terrainium enter".to_string());
        aliases.insert("texit".to_string(), "terrainium exit".to_string());

        let constructors: Commands = Commands::new(
            vec![Command::new(
                "/bin/echo".to_string(),
                vec!["entering terrain".to_string()],
                None,
            )],
            vec![],
        );

        let destructors: Commands = Commands::new(
            vec![Command::new(
                "/bin/echo".to_string(),
                vec!["exiting terrain".to_string()],
                None,
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
