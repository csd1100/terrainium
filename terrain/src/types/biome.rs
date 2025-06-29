use std::collections::BTreeMap;

use terrainium_lib::command::Command;

use crate::constants::{
    EDITOR, ENV_VAR, NESTED_POINTER, NULL_POINTER, PAGER, POINTER_ENV_VAR, TENTER, TEXIT,
};
use crate::types::commands::Commands;

/// [Biome] is a most basic unit of environment
/// stores environment variables, alias, constructors,
/// and destructors
pub struct Biome {
    name: String,
    envs: BTreeMap<String, String>,
    aliases: BTreeMap<String, String>,
    constructors: Commands,
    destructors: Commands,
}

impl Biome {
    /// Constructs a new [Biome]
    pub fn new(
        name: String,
        envs: BTreeMap<String, String>,
        aliases: BTreeMap<String, String>,
        constructors: Commands,
        destructors: Commands,
    ) -> Self {
        Self {
            name,
            envs,
            aliases,
            constructors,
            destructors,
        }
    }

    /// Returns the name of the biome.
    ///
    /// If it is called on main terrain "none" is returned
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Merges a [Biome] with another.
    ///
    /// Overrides values within itself with values from the other.
    /// Call this for main terrain. So biome will override the main
    /// terrain.
    pub fn merged(mut self, other: Biome) -> Biome {
        self.merge_envs(other.envs);
        self.merge_aliases(other.aliases);
        self.append_constructors(other.constructors);
        self.append_destructors(other.destructors);

        Biome::new(
            other.name,
            self.envs,
            self.aliases,
            self.constructors,
            self.destructors,
        )
    }

    /// Merges environment variables from other map.
    ///
    /// Overrides values from [self] with the other.
    fn merge_envs(&mut self, other: BTreeMap<String, String>) {
        self.envs.extend(other)
    }

    /// Merges aliases from other map.
    ///
    /// Overrides values from [self] with the other.
    fn merge_aliases(&mut self, other: BTreeMap<String, String>) {
        self.aliases.extend(other)
    }

    /// Appends constructors from other constructors.
    ///
    /// Other constructors are added after [self]'s
    fn append_constructors(&mut self, other: Commands) {
        self.constructors.append(other)
    }

    /// Appends destructors from other destructors.
    ///
    /// Other destructors are added after [self]'s
    fn append_destructors(&mut self, other: Commands) {
        self.destructors.append(other)
    }

    pub fn example(name: String) -> Biome {
        Self {
            name,
            envs: example_terrain_envs(),
            aliases: example_terrain_aliases(),
            constructors: example_terrain_constructors(),
            destructors: example_terrain_destructors(),
        }
    }
}

/// Environment Variables used by main terrain in example [Terrain]
fn example_terrain_envs() -> BTreeMap<String, String> {
    let mut envs: BTreeMap<String, String> = BTreeMap::new();
    envs.insert(EDITOR.to_string(), "vim".to_string());
    envs.insert(ENV_VAR.to_string(), "env_val".to_string());
    envs.insert(
        NESTED_POINTER.to_string(),
        "${POINTER_ENV_VAR}-${ENV_VAR}-${NULL_POINTER}".to_string(),
    );
    envs.insert(NULL_POINTER.to_string(), "${NULL}".to_string());
    envs.insert(PAGER.to_string(), "less".to_string());
    envs.insert(POINTER_ENV_VAR.to_string(), "${ENV_VAR}".to_string());
    envs
}

/// Aliases used by main terrain in example [Terrain]
fn example_terrain_aliases() -> BTreeMap<String, String> {
    let mut aliases: BTreeMap<String, String> = BTreeMap::new();
    aliases.insert(TENTER.to_string(), "terrain enter".to_string());
    aliases.insert(TEXIT.to_string(), "terrain exit".to_string());
    aliases
}

/// Constructors used by main terrain in example [Terrain]
fn example_terrain_constructors() -> Commands {
    Commands::new(
        vec![Command::new(
            "/bin/echo".to_string(),
            vec!["entering terrain".to_string()],
            None,
        )],
        vec![],
    )
}

/// Destructors used by main terrain in example [Terrain]
fn example_terrain_destructors() -> Commands {
    Commands::new(
        vec![Command::new(
            "/bin/echo".to_string(),
            vec!["exiting terrain".to_string()],
            None,
        )],
        vec![],
    )
}
