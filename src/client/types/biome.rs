use crate::client::types::commands::Commands;
use crate::client::validation::{validate_identifiers, IdentifierType, ValidationResults};
use crate::common::constants::{
    ALIASES, BACKGROUND, CONSTRUCTORS, DESTRUCTORS, DOES_NOT_EXIST, ENVS, FOREGROUND,
};
use crate::common::types::command::{Command, OperationType};
use anyhow::{Context, Result};
use regex::Regex;
#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use toml_edit::{value, Array, Item, Table};

fn replace_key(table: &mut Item, old_key: &str, new_key: &str) {
    let value = table.as_table_mut().unwrap().remove(old_key);
    table[new_key] = value.unwrap();
}

fn get_commands(commands: &Commands) -> String {
    let cmds = |c: &Vec<Command>| -> String {
        c.iter().map(|cmd| format!("{: <8}{}\n", "", cmd)).collect()
    };
    // adds 4 spaces behind
    let mut result = format!("{: <4}foreground:\n", "");
    result += &cmds(commands.foreground());
    result += &format!("{: <4}background:\n", "");
    result += &cmds(commands.background());
    result
}

fn get_pairs<S: Into<String> + std::fmt::Display>(map: &BTreeMap<S, S>) -> String {
    map.iter()
        .map(|(k, v)| format!("{: <4}{k}=\"{v}\"\n", ""))
        .collect::<String>()
}

fn get_filtered<'a>(
    map: &'a BTreeMap<String, String>,
    filter: &'a [String],
) -> BTreeMap<&'a str, &'a str> {
    let mut result = BTreeMap::<&str, &str>::new();
    filter.iter().for_each(|key| {
        if let Some(value) = map.get(key) {
            result.insert(key, value);
        } else {
            result.insert(key, DOES_NOT_EXIST);
        }
    });
    result
}

#[cfg_attr(feature = "terrain-schema", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Biome {
    #[serde(skip)]
    name: String,
    envs: BTreeMap<String, String>,
    aliases: BTreeMap<String, String>,
    constructors: Commands,
    destructors: Commands,
}

impl Biome {
    pub fn new(
        name: String,
        envs: BTreeMap<String, String>,
        aliases: BTreeMap<String, String>,
        constructors: Commands,
        destructors: Commands,
    ) -> Self {
        Biome {
            name,
            envs,
            aliases,
            constructors,
            destructors,
        }
    }

    pub(crate) fn new_toml() -> Table {
        let mut commands = Table::new();
        commands.insert(FOREGROUND, Array::new().into());
        commands.insert(BACKGROUND, Array::new().into());

        let mut new_biome = Table::new();
        new_biome.insert(ENVS, Table::new().into());
        new_biome.insert(ALIASES, Table::new().into());
        new_biome.insert(CONSTRUCTORS, commands.clone().into());
        new_biome.insert(DESTRUCTORS, commands.into());

        new_biome.fmt();
        new_biome
    }

    pub(crate) fn name(&self) -> String {
        self.name.clone()
    }

    pub(crate) fn aliases(&self) -> &BTreeMap<String, String> {
        &self.aliases
    }

    pub(crate) fn aliases_str(&self, filter: Option<&[String]>) -> String {
        let header = String::from("Aliases:");

        let body = if let Some(filter) = filter {
            get_pairs(&get_filtered(&self.aliases, filter))
        } else {
            get_pairs(&self.aliases)
        };

        if body.is_empty() {
            return String::new();
        }

        format!("{header}\n{body}")
    }

    pub(crate) fn envs(&self) -> &BTreeMap<String, String> {
        &self.envs
    }

    pub(crate) fn envs_str(&self, filter: Option<&[String]>) -> String {
        let header = String::from("Environment Variables:");

        let body = if let Some(filter) = filter {
            get_pairs(&get_filtered(&self.envs, filter))
        } else {
            get_pairs(&self.envs)
        };

        if body.is_empty() {
            return String::new();
        }

        format!("{header}\n{body}")
    }

    pub(crate) fn constructors(&self) -> &Commands {
        &self.constructors
    }

    pub(crate) fn constructors_str(&self) -> String {
        let header = String::from("Constructors:");
        let body = get_commands(&self.constructors);
        format!("{header}\n{body}")
    }

    pub(crate) fn destructors(&self) -> &Commands {
        &self.destructors
    }

    pub(crate) fn destructors_str(&self) -> String {
        let header = String::from("Destructors:");
        let body = get_commands(&self.destructors);
        format!("{header}\n{body}")
    }

    pub(crate) fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub(crate) fn set_envs(&mut self, envs: BTreeMap<String, String>) {
        self.envs = envs;
    }

    pub(crate) fn insert_env(&mut self, key: String, value: String) {
        self.envs.insert(key, value);
    }

    pub(crate) fn insert_foreground_constructor(&mut self, idx: usize, command: Command) {
        self.constructors.foreground_mut().insert(idx, command);
    }

    pub(crate) fn insert_background_constructor(&mut self, idx: usize, command: Command) {
        self.constructors.background_mut().insert(idx, command);
    }

    pub(crate) fn insert_foreground_destructor(&mut self, idx: usize, command: Command) {
        self.destructors.foreground_mut().insert(idx, command);
    }

    pub(crate) fn insert_background_destructor(&mut self, idx: usize, command: Command) {
        self.destructors.background_mut().insert(idx, command);
    }

    pub(crate) fn remove_foreground_constructor(&mut self, command: &Command) -> Option<usize> {
        let idx = self
            .constructors
            .foreground()
            .iter()
            .position(|v| v == command);
        if let Some(idx) = idx {
            self.constructors.foreground_mut().remove(idx);
        }
        idx
    }

    pub(crate) fn remove_background_constructor(&mut self, command: &Command) -> Option<usize> {
        let idx = self
            .constructors
            .background()
            .iter()
            .position(|v| v == command);
        if let Some(idx) = idx {
            self.constructors.background_mut().remove(idx);
        }
        idx
    }

    pub(crate) fn remove_foreground_destructor(&mut self, command: &Command) -> Option<usize> {
        let idx = self
            .destructors
            .foreground()
            .iter()
            .position(|v| v == command);
        if let Some(idx) = idx {
            self.destructors.foreground_mut().remove(idx);
        }
        idx
    }
    pub(crate) fn remove_background_destructor(&mut self, command: &Command) -> Option<usize> {
        let idx = self
            .destructors
            .background()
            .iter()
            .position(|v| v == command);
        if let Some(idx) = idx {
            self.destructors.background_mut().remove(idx);
        }
        idx
    }

    pub(crate) fn append_envs(&mut self, envs: BTreeMap<String, String>) {
        self.envs.extend(envs);
    }

    pub(crate) fn merge(&self, another: &Biome) -> Biome {
        Biome::new(
            another.name(),
            self.merge_envs(another),
            self.merge_aliases(another),
            self.merge_constructors(another),
            self.merge_destructors(another),
        )
    }

    pub(crate) fn merge_destructors(&self, another: &Biome) -> Commands {
        let mut destructors = self.destructors.clone();
        let mut another_destructors = another.destructors.clone();

        destructors.append(&mut another_destructors);
        destructors
    }

    pub(crate) fn merge_constructors(&self, another: &Biome) -> Commands {
        let mut constructors = self.constructors.clone();
        let mut another_constructors = another.constructors.clone();

        constructors.append(&mut another_constructors);
        constructors
    }

    pub(crate) fn merge_aliases(&self, another: &Biome) -> BTreeMap<String, String> {
        let mut aliases = self.aliases.clone();
        let mut another_aliases = another.aliases.clone();

        aliases.append(&mut another_aliases);
        aliases
    }

    pub(crate) fn merge_envs(&self, another: &Biome) -> BTreeMap<String, String> {
        let mut envs = self.envs.clone();
        let mut another_envs = another.envs.clone();

        envs.append(&mut another_envs);
        envs
    }

    pub(crate) fn get_envs_to_substitute(str_to_parse: &str) -> Vec<String> {
        let mut result = vec![];
        let re =
            Regex::new(r"\$\{(.*?)}").expect("environment variable reference regex to be parsed");
        for (_, [val]) in re.captures_iter(str_to_parse).map(|c| c.extract()) {
            result.push(val.to_string());
        }
        result
    }

    pub(crate) fn recursive_substitute_envs(
        envs: &BTreeMap<String, String>,
        result_string: String,
        envs_to_substitute: Vec<String>,
    ) -> String {
        let mut envs_to_substitute = envs_to_substitute;
        let mut result_string = result_string;

        // recurse till envs_to_substitute is not empty
        if !envs_to_substitute.is_empty() {
            let env = envs_to_substitute.pop().unwrap();

            if envs.contains_key(&env) || std::env::var(&env).is_ok() {
                let env_val = if let Some(env_val) = envs.get(&env) {
                    env_val.to_string()
                } else {
                    std::env::var(&env).unwrap()
                };

                // if value present in terrain envs or system envs replace the value
                let value_to_replace = format!("${{{env}}}");
                result_string = result_string.replace(&value_to_replace, &env_val);

                // if the new value is also env ref add that to substitute list
                let new_env_to_substitute = Self::get_envs_to_substitute(&env_val);
                envs_to_substitute.extend(new_env_to_substitute);
            }
            return Self::recursive_substitute_envs(envs, result_string, envs_to_substitute);
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
                    Self::recursive_substitute_envs(biome_envs, value.clone(), envs_to_substitute),
                )
            })
            .collect();

        self.set_envs(BTreeMap::from_iter(substituted_envs));
    }

    pub(crate) fn substitute_cwd(&mut self, terrain_dir: &Path) -> Result<()> {
        self.constructors
            .substitute_cwd(terrain_dir, &self.envs)
            .context("failed to substitute cwd for constructors")?;
        self.destructors
            .substitute_cwd(terrain_dir, &self.envs)
            .context("failed to substitute cwd for destructors")
    }

    pub(crate) fn validate<'a>(
        &'a self,
        biome_name: &'a str,
        terrain_dir: &'a Path,
    ) -> ValidationResults<'a> {
        let mut result = ValidationResults::new(false, HashSet::new());
        result.append(self.validate_envs(biome_name));
        result.append(self.validate_aliases(biome_name));
        result.append(self.validate_constructors(biome_name, terrain_dir));
        result.append(self.validate_destructors(biome_name, terrain_dir));
        result
    }

    fn validate_envs<'a>(&'a self, biome_name: &'a str) -> ValidationResults<'a> {
        validate_identifiers(IdentifierType::Env, &self.envs, biome_name)
    }

    fn validate_aliases<'a>(&'a self, biome_name: &'a str) -> ValidationResults<'a> {
        validate_identifiers(IdentifierType::Alias, &self.aliases, biome_name)
    }

    fn validate_constructors<'a>(
        &'a self,
        biome_name: &'a str,
        terrain_dir: &'a Path,
    ) -> ValidationResults<'a> {
        self.constructors
            .validate_commands(biome_name, OperationType::Constructor, terrain_dir)
    }

    fn validate_destructors<'a>(
        &'a self,
        biome_name: &'a str,
        terrain_dir: &'a Path,
    ) -> ValidationResults<'a> {
        self.destructors
            .validate_commands(biome_name, OperationType::Destructor, terrain_dir)
    }

    pub(crate) fn replace_env_key(&mut self, key: &str, fixed: &str) {
        let value = self.envs.remove(key).unwrap();
        self.envs.insert(fixed.to_string(), value);
    }

    pub(crate) fn replace_env_key_toml(biome_toml: &mut Item, key: &str, fixed: &str) {
        replace_key(&mut biome_toml[ENVS], key, fixed);
    }

    pub(crate) fn replace_alias_key(&mut self, key: &str, fixed: &str) {
        let value = self.aliases.remove(key).unwrap();
        self.aliases.insert(fixed.to_string(), value);
    }

    pub(crate) fn replace_alias_key_toml(biome_toml: &mut Item, key: &str, fixed: &str) {
        replace_key(&mut biome_toml[ALIASES], key, fixed);
    }

    pub(crate) fn replace_command_exe_toml(
        biome_toml: &mut Item,
        operation_type: &str,
        command_type: &str,
        idx: usize,
        fixed: &str,
    ) {
        assert_eq!(
            biome_toml[operation_type][command_type][idx]["exe"]
                .as_str()
                .unwrap()
                .trim(),
            fixed
        );
        biome_toml[operation_type][command_type][idx]["exe"] = value(fixed);
    }

    pub fn example(name: String) -> Self {
        Biome::new(
            name,
            example_envs(),
            example_aliases(),
            example_constructors(),
            example_destructors(),
        )
    }
}

pub(crate) fn example_envs() -> BTreeMap<String, String> {
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
    envs
}

pub(crate) fn example_aliases() -> BTreeMap<String, String> {
    let mut aliases: BTreeMap<String, String> = BTreeMap::new();
    aliases.insert("tenter".to_string(), "terrainium enter".to_string());
    aliases.insert("texit".to_string(), "terrainium exit".to_string());
    aliases
}

pub(crate) fn example_constructors() -> Commands {
    Commands::new(
        vec![Command::new(
            "/bin/echo".to_string(),
            vec!["entering terrain".to_string()],
            None,
            None,
        )],
        vec![],
    )
}

pub(crate) fn example_destructors() -> Commands {
    Commands::new(
        vec![Command::new(
            "/bin/echo".to_string(),
            vec!["exiting terrain".to_string()],
            None,
            None,
        )],
        vec![],
    )
}

#[cfg(test)]
impl Biome {
    pub(crate) fn add_envs(&mut self, envs: Vec<(&str, &str)>) {
        envs.into_iter().for_each(|(k, v)| {
            self.envs.insert(k.to_string(), v.to_string());
        })
    }

    pub(crate) fn add_bg_constructors(&mut self, commands: Vec<Command>) {
        self.constructors.background_mut().extend(commands);
    }

    pub(crate) fn add_bg_destructors(&mut self, commands: Vec<Command>) {
        self.destructors.background_mut().extend(commands);
    }

    #[allow(dead_code)]
    pub(crate) fn add_fg_constructors(&mut self, commands: Vec<Command>) {
        self.constructors.foreground_mut().extend(commands);
    }

    #[allow(dead_code)]
    pub(crate) fn add_fg_destructors(&mut self, commands: Vec<Command>) {
        self.destructors.foreground_mut().extend(commands);
    }

    pub(crate) fn set_constructors(&mut self, constructors: Commands) {
        self.constructors = constructors;
    }

    pub(crate) fn set_destructors(&mut self, destructors: Commands) {
        self.destructors = destructors;
    }

    pub(crate) fn set_aliases(&mut self, aliases: BTreeMap<String, String>) {
        self.aliases = aliases;
    }
}
