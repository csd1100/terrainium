use crate::client::types::biome::Biome;
use crate::client::types::command::Command;
use crate::client::types::commands::Commands;
use crate::client::validation::{
    ValidationError, ValidationMessageLevel, ValidationResult, ValidationResults,
};
use anyhow::{anyhow, Context, Result};
use log::{debug, error, info, warn};
#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg_attr(feature = "terrain-schema", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct AutoApply {
    enabled: bool,
    background: bool,
    replace: bool,
}

impl AutoApply {
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            background: false,
            replace: false,
        }
    }

    pub fn background() -> Self {
        Self {
            enabled: true,
            background: true,
            replace: false,
        }
    }

    pub fn replace() -> Self {
        Self {
            enabled: true,
            background: false,
            replace: true,
        }
    }

    pub fn all() -> Self {
        Self {
            enabled: true,
            background: true,
            replace: true,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled && !self.background && !self.replace
    }

    pub fn is_background(&self) -> bool {
        self.enabled && self.background
    }

    pub fn is_replace(&self) -> bool {
        self.enabled && self.replace
    }

    pub fn is_all(&self) -> bool {
        self.enabled && self.background && self.replace
    }
}

impl From<AutoApply> for String {
    fn from(value: AutoApply) -> Self {
        if value.is_all() {
            "all"
        } else if value.is_enabled() {
            "enabled"
        } else if value.is_replace() {
            "replaced"
        } else if value.is_background() {
            "background"
        } else {
            "off"
        }
        .to_string()
    }
}

pub fn schema_url() -> String {
    "https://raw.githubusercontent.com/csd1100/terrainium/main/schema/terrain-schema.json"
        .to_string()
}

#[cfg_attr(feature = "terrain-schema", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Clone)]
pub struct Terrain {
    #[serde(default = "schema_url", rename(serialize = "$schema"))]
    schema: String,

    name: String,
    auto_apply: AutoApply,
    terrain: Biome,
    biomes: BTreeMap<String, Biome>,
    default_biome: Option<String>,
}

impl Terrain {
    pub fn new(
        terrain: Biome,
        biomes: BTreeMap<String, Biome>,
        default_biome: Option<String>,
        auto_apply: AutoApply,
    ) -> Self {
        let name = std::env::current_dir()
            .expect("failed to get current directory")
            .file_name()
            .expect("failed to get current directory name")
            .to_str()
            .expect("failed to convert directory name to string")
            .to_string();

        Terrain {
            name,
            schema: schema_url(),
            auto_apply,
            terrain,
            biomes,
            default_biome,
        }
    }

    pub fn default_biome(&self) -> &Option<String> {
        &self.default_biome
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn terrain(&self) -> &Biome {
        &self.terrain
    }

    pub fn terrain_mut(&mut self) -> &mut Biome {
        &mut self.terrain
    }

    pub fn biomes(&self) -> &BTreeMap<String, Biome> {
        &self.biomes
    }

    pub fn auto_apply(&self) -> &AutoApply {
        &self.auto_apply
    }

    pub fn set_auto_apply(&mut self, auto_apply: AutoApply) {
        self.auto_apply = auto_apply;
    }

    pub fn merged(&self, selected_biome: &Option<String>) -> Result<Biome> {
        let selected = self.select_biome(selected_biome)?.1;
        if selected == &self.terrain {
            Ok(self.terrain.clone())
        } else {
            Ok(self.terrain.merge(selected))
        }
    }

    pub(crate) fn select_biome(&self, selected: &Option<String>) -> Result<(String, &Biome)> {
        let selected = match selected {
            None => self.default_biome.clone(),
            Some(selected) => Some(selected.clone()),
        };
        match selected {
            None => Ok(("none".to_string(), &self.terrain)),
            Some(selected) => {
                if selected == "none" {
                    Ok(("none".to_string(), &self.terrain))
                } else if let Some(biome) = self.biomes.get(&selected) {
                    Ok((selected, biome))
                } else {
                    Err(anyhow!("the biome {:?} does not exists", selected))
                }
            }
        }
    }

    fn validate(&self) -> Result<ValidationResults, ValidationError> {
        // validate terrain
        let mut results = self.terrain.validate("none");

        // all biomes
        self.biomes
            .iter()
            .for_each(|(biome_name, biome)| results.append(&mut biome.validate(biome_name)));

        if results
            .results_ref()
            .iter()
            .any(|val| val.level == ValidationMessageLevel::Error)
        {
            return Err(ValidationError {
                messages: results.results(),
            });
        }

        Ok(results)
    }

    pub fn from_toml(toml_str: String) -> Result<Self> {
        toml::from_str(&toml_str).context("failed to parse terrain from toml")
        // TODO: add validation here
    }

    pub fn to_toml(&self) -> Result<String> {
        // TODO: add validation here
        toml::to_string(&self).context("failed to convert terrain to toml")
    }

    pub(crate) fn set_default(&mut self, new_default: String) {
        self.default_biome = Some(new_default);
    }

    pub(crate) fn update(&mut self, biome_name: String, updated: Biome) {
        if biome_name == "none" {
            self.terrain = updated
        } else {
            self.biomes.insert(biome_name, updated);
        }
    }

    pub fn example() -> Self {
        let terrain = Biome::example();

        let mut biome_envs: BTreeMap<String, String> = BTreeMap::new();
        biome_envs.insert("EDITOR".to_string(), "nvim".to_string());
        biome_envs.insert("ENV_VAR".to_string(), "overridden_env_val".to_string());

        let mut biome_aliases: BTreeMap<String, String> = BTreeMap::new();
        biome_aliases.insert(
            "tenter".to_string(),
            "terrainium enter --biome example_biome".to_string(),
        );

        let biome_constructors: Commands = Commands::new(
            vec![Command::new(
                "/bin/echo".to_string(),
                vec!["entering biome example_biome".to_string()],
                None,
            )],
            vec![Command::new(
                "/bin/bash".to_string(),
                vec![
                    "-c".to_string(),
                    "$PWD/tests/scripts/print_num_for_10_sec".to_string(),
                ],
                None,
            )],
        );

        let biome_destructors: Commands = Commands::new(
            vec![Command::new(
                "/bin/echo".to_string(),
                vec!["exiting biome example_biome".to_string()],
                None,
            )],
            vec![Command::new(
                "/bin/bash".to_string(),
                vec![
                    "-c".to_string(),
                    "$PWD/tests/scripts/print_num_for_10_sec".to_string(),
                ],
                None,
            )],
        );

        let example_biome = Biome::new(
            biome_envs,
            biome_aliases,
            biome_constructors,
            biome_destructors,
        );

        let mut biomes: BTreeMap<String, Biome> = BTreeMap::new();
        biomes.insert(String::from("example_biome"), example_biome);

        Terrain::new(
            terrain,
            biomes,
            Some(String::from("example_biome")),
            AutoApply {
                enabled: false,
                background: false,
                replace: false,
            },
        )
    }
}

impl Default for Terrain {
    fn default() -> Self {
        Terrain::new(
            Biome::default(),
            BTreeMap::new(),
            None,
            AutoApply {
                enabled: false,
                background: false,
                replace: false,
            },
        )
    }
}

#[cfg(test)]
pub mod tests {
    use crate::client::types::biome::Biome;
    use crate::client::types::command::Command;
    use crate::client::types::commands::Commands;
    use crate::client::types::terrain::Terrain;
    use crate::client::validation::{ValidationMessageLevel, ValidationResult};
    use std::collections::{BTreeMap, HashSet};

    pub fn force_set_invalid_default_biome(terrain: &mut Terrain, default_biome: Option<String>) {
        terrain.default_biome = default_biome;
    }

    pub fn add_biome(terrain: &mut Terrain, name: String, biome: Biome) {
        terrain.biomes.insert(name, biome);
    }

    pub fn get_test_biome(name: String, editor: String) -> Biome {
        let mut biome_envs: BTreeMap<String, String> = BTreeMap::new();
        biome_envs.insert("EDITOR".to_string(), editor);

        let mut biome_aliases: BTreeMap<String, String> = BTreeMap::new();
        biome_aliases.insert(
            "tenter".to_string(),
            "terrainium enter --biome ".to_string() + &name,
        );
        let biome_constructor_foreground: Vec<Command> = vec![Command::new(
            "/bin/echo".to_string(),
            vec!["entering biome ".to_string() + &name],
            None,
        )];
        let biome_constructor_background: Vec<Command> = vec![];
        let biome_destructor_foreground: Vec<Command> = vec![Command::new(
            "/bin/echo".to_string(),
            vec!["exiting biome ".to_string() + &name],
            None,
        )];
        let biome_destructor_background: Vec<Command> = vec![];

        let biome_constructor =
            Commands::new(biome_constructor_foreground, biome_constructor_background);
        let biome_destructor =
            Commands::new(biome_destructor_foreground, biome_destructor_background);
        Biome::new(
            biome_envs,
            biome_aliases,
            biome_constructor,
            biome_destructor,
        )
    }

    #[test]
    fn validate_envs() {
        let mut terrain = Terrain::default();
        let mut biome = Biome::default();

        let mut envs = BTreeMap::<String, String>::new();
        envs.insert("".to_string(), "VALUE_WITHOUT_SPACES".to_string());
        envs.insert(
            "TEST_ENV_WITHOUT_SPACES".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        envs.insert(
            "TEST_VALUE_WITH_SPACES".to_string(),
            "VALUE WITH SPACES".to_string(),
        );
        envs.insert(
            "TEST ENV WITH SPACES".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        envs.insert(
            " ENV_WITH_LEADING_SPACES".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        envs.insert(
            "ENV_WITH_TRAILING_SPACES ".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        envs.insert(
            "1ENV_STARTING_WITH_NUM".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        envs.insert(
            "ALPHA_NUMERIC_123".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        envs.insert(
            "alpha_numeric_123".to_string(),
            "value_without_spaces".to_string(),
        );
        envs.insert(
            "ENV-WITH-INVALID-#.(".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        envs.insert(
            " 1INVALID-#. ( ".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );

        terrain.terrain_mut().set_envs(envs.clone());
        biome.set_envs(envs);
        terrain.update("test_biome".to_string(), biome);

        let validation_result = terrain.validate().expect_err("expected validation error");

        let messages: HashSet<ValidationResult> =
            validation_result.messages.iter().cloned().collect();

        assert_eq!(messages.len(), 22);
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "empty environment variable identifier is not allowed".to_string(),
            target: "none".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message:
                "environment variable identifier `TEST ENV WITH SPACES` is invalid as it contains spaces"
                    .to_string(),
            target: "none".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Info,
            message:
                "trimming spaces from environment variable identifier: ` ENV_WITH_LEADING_SPACES`"
                    .to_string(),
            target: "none".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Info,
            message:
                "trimming spaces from environment variable identifier: `ENV_WITH_TRAILING_SPACES `"
                    .to_string(),
            target: "none".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message:
                "environment variable identifier `1ENV_STARTING_WITH_NUM` cannot start with number"
                    .to_string(),
            target: "none".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "environment variable identifier `ENV-WITH-INVALID-#.(` contains invalid characters. environment variable name can only include [a-zA-Z0-9_] characters.".to_string(),
            target: "none".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message:
                "environment variable identifier `1INVALID-#. (` is invalid as it contains spaces"
                    .to_string(),
            target: "none".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Info,
            message: "trimming spaces from environment variable identifier: ` 1INVALID-#. ( `"
                .to_string(),
            target: "none".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Info,
            message: "trimming spaces from environment variable identifier: ` 1INVALID-#. ( `"
                .to_string(),
            target: "none".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "environment variable identifier `1INVALID-#. (` cannot start with number"
                .to_string(),
            target: "none".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "environment variable identifier `1INVALID-#. (` contains invalid characters. environment variable name can only include [a-zA-Z0-9_] characters.".to_string(),
            target: "none".to_string(),
        }));

        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "empty environment variable identifier is not allowed".to_string(),
            target: "test_biome".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message:
                "environment variable identifier `TEST ENV WITH SPACES` is invalid as it contains spaces"
                    .to_string(),
            target: "test_biome".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Info,
            message:
                "trimming spaces from environment variable identifier: ` ENV_WITH_LEADING_SPACES`"
                    .to_string(),
            target: "test_biome".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Info,
            message:
                "trimming spaces from environment variable identifier: `ENV_WITH_TRAILING_SPACES `"
                    .to_string(),
            target: "test_biome".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message:
                "environment variable identifier `1ENV_STARTING_WITH_NUM` cannot start with number"
                    .to_string(),
            target: "test_biome".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "environment variable identifier `ENV-WITH-INVALID-#.(` contains invalid characters. environment variable name can only include [a-zA-Z0-9_] characters.".to_string(),
            target: "test_biome".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message:
                "environment variable identifier `1INVALID-#. (` is invalid as it contains spaces"
                    .to_string(),
            target: "test_biome".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Info,
            message: "trimming spaces from environment variable identifier: ` 1INVALID-#. ( `"
                .to_string(),
            target: "test_biome".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Info,
            message: "trimming spaces from environment variable identifier: ` 1INVALID-#. ( `"
                .to_string(),
            target: "test_biome".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "environment variable identifier `1INVALID-#. (` cannot start with number"
                .to_string(),
            target: "test_biome".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "environment variable identifier `1INVALID-#. (` contains invalid characters. environment variable name can only include [a-zA-Z0-9_] characters.".to_string(),
            target: "test_biome".to_string(),
        }));
    }

    #[test]
    fn validate_aliases() {
        let mut terrain = Terrain::default();
        let mut biome = Biome::default();

        let mut aliases = BTreeMap::<String, String>::new();
        aliases.insert("".to_string(), "value_without_spaces".to_string());
        aliases.insert(
            "TEST_ALIAS_WITHOUT_SPACES".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        aliases.insert(
            "TEST_VALUE_WITH_SPACES".to_string(),
            "VALUE WITH SPACES".to_string(),
        );
        aliases.insert(
            "TEST ALIAS WITH SPACES".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        aliases.insert(
            " ALIAS_WITH_LEADING_SPACES".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        aliases.insert(
            "ALIAS_WITH_TRAILING_SPACES ".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        aliases.insert(
            "1ALIAS_STARTING_WITH_NUM".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        aliases.insert(
            "ALPHA_NUMERIC_123".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        aliases.insert(
            "alpha_numeric_123".to_string(),
            "value_without_spaces".to_string(),
        );
        aliases.insert(
            "ALIAS-WITH-INVALID-#.(".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        aliases.insert(
            " 1INVALID-#. ( ".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );

        terrain.terrain_mut().set_aliases(aliases.clone());
        biome.set_aliases(aliases);
        terrain.update("test_biome".to_string(), biome);

        let validation_result = terrain.validate().expect_err("expected validation error");

        let messages: HashSet<ValidationResult> =
            validation_result.messages.iter().cloned().collect();

        assert_eq!(messages.len(), 22);
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "empty alias identifier is not allowed".to_string(),
            target: "none".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "alias identifier `TEST ALIAS WITH SPACES` is invalid as it contains spaces"
                .to_string(),
            target: "none".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Info,
            message: "trimming spaces from alias identifier: ` ALIAS_WITH_LEADING_SPACES`"
                .to_string(),
            target: "none".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Info,
            message: "trimming spaces from alias identifier: `ALIAS_WITH_TRAILING_SPACES `"
                .to_string(),
            target: "none".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "alias identifier `1ALIAS_STARTING_WITH_NUM` cannot start with number"
                .to_string(),
            target: "none".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "alias identifier `ALIAS-WITH-INVALID-#.(` contains invalid characters. alias name can only include [a-zA-Z0-9_] characters.".to_string(),
            target: "none".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "alias identifier `1INVALID-#. (` is invalid as it contains spaces"
                .to_string(),
            target: "none".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Info,
            message: "trimming spaces from alias identifier: ` 1INVALID-#. ( `".to_string(),
            target: "none".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Info,
            message: "trimming spaces from alias identifier: ` 1INVALID-#. ( `".to_string(),
            target: "none".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "alias identifier `1INVALID-#. (` cannot start with number".to_string(),
            target: "none".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "alias identifier `1INVALID-#. (` contains invalid characters. alias name can only include [a-zA-Z0-9_] characters.".to_string(),
            target: "none".to_string(),
        }));

        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "empty alias identifier is not allowed".to_string(),
            target: "test_biome".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "alias identifier `TEST ALIAS WITH SPACES` is invalid as it contains spaces"
                .to_string(),
            target: "test_biome".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Info,
            message: "trimming spaces from alias identifier: ` ALIAS_WITH_LEADING_SPACES`"
                .to_string(),
            target: "test_biome".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Info,
            message: "trimming spaces from alias identifier: `ALIAS_WITH_TRAILING_SPACES `"
                .to_string(),
            target: "test_biome".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "alias identifier `1ALIAS_STARTING_WITH_NUM` cannot start with number"
                .to_string(),
            target: "test_biome".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "alias identifier `ALIAS-WITH-INVALID-#.(` contains invalid characters. alias name can only include [a-zA-Z0-9_] characters.".to_string(),
            target: "test_biome".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "alias identifier `1INVALID-#. (` is invalid as it contains spaces"
                .to_string(),
            target: "test_biome".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Info,
            message: "trimming spaces from alias identifier: ` 1INVALID-#. ( `".to_string(),
            target: "test_biome".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Info,
            message: "trimming spaces from alias identifier: ` 1INVALID-#. ( `".to_string(),
            target: "test_biome".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "alias identifier `1INVALID-#. (` cannot start with number".to_string(),
            target: "test_biome".to_string(),
        }));
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Error,
            message: "alias identifier `1INVALID-#. (` contains invalid characters. alias name can only include [a-zA-Z0-9_] characters.".to_string(),
            target: "test_biome".to_string(),
        }));
    }
}
