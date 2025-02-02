use crate::client::types::biome::Biome;
use crate::client::types::command::Command;
use crate::client::types::commands::Commands;
use crate::client::validation::{ValidationError, ValidationMessageLevel, ValidationResults};
use anyhow::{anyhow, Context as AnyhowContext, Result};
#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

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

    fn validate(&self, terrain_dir: &Path) -> Result<ValidationResults, ValidationError> {
        // validate terrain
        let mut results = self.terrain.validate("none", terrain_dir);

        // all biomes
        self.biomes.iter().for_each(|(biome_name, biome)| {
            results.append(&mut biome.validate(biome_name, terrain_dir))
        });

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
    use crate::client::utils::{restore_env_var, set_env_var};
    use crate::client::validation::{ValidationMessageLevel, ValidationResult};
    use serial_test::serial;
    use std::collections::{BTreeMap, HashSet};
    use std::fs::{create_dir_all, metadata, set_permissions, write};
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use tempfile::tempdir;

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
    fn validate_aliases_and_envs() {
        let mut terrain = Terrain::default();
        let mut biome = Biome::default();

        let mut map = BTreeMap::<String, String>::new();
        map.insert("".to_string(), "VALUE_WITHOUT_SPACES".to_string());
        map.insert(
            "TEST_WITHOUT_SPACES".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        map.insert(
            "TEST_VALUE_WITH_SPACES".to_string(),
            "VALUE WITH SPACES".to_string(),
        );
        map.insert(
            "TEST WITH SPACES".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        map.insert(
            " WITH_LEADING_SPACES".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        map.insert(
            "WITH_TRAILING_SPACES ".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        map.insert(
            "1STARTING_WITH_NUM".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        map.insert(
            "ALPHA_NUMERIC_123".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        map.insert(
            "alpha_numeric_123".to_string(),
            "value_without_spaces".to_string(),
        );
        map.insert(
            "WITH-INVALID-#.(".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        map.insert(
            " 1INVALID-#. ( ".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );

        terrain.terrain_mut().set_envs(map.clone());
        terrain.terrain_mut().set_aliases(map.clone());
        biome.set_envs(map.clone());
        biome.set_aliases(map);
        terrain.update("test_biome".to_string(), biome);

        let validation_result = terrain
            .validate(&PathBuf::new())
            .expect_err("expected validation error");

        let messages: HashSet<ValidationResult> =
            validation_result.messages.iter().cloned().collect();

        assert_eq!(messages.len(), 44);

        ["none", "test_biome"].iter().for_each(|target| {
            ["env", "alias"].iter().for_each(|iterator_type| {
                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Error,
                    message: "empty identifier is not allowed".to_string(),
                    target: format!("{}({})", target, iterator_type),
                }), "failed to validate empty identifier message for {}({})", target, iterator_type);

                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Error,
                    message:
                    "identifier `TEST WITH SPACES` is invalid as it contains spaces"
                        .to_string(),
                    target: format!("{}({})", target, iterator_type),
                }), "failed to validate identifier with spaces message for {}({})", target, iterator_type);

                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Info,
                    message:
                    "trimming spaces from identifier: ` WITH_LEADING_SPACES`"
                        .to_string(),
                    target: format!("{}({})", target, iterator_type),
                }), "failed to validate trimming leading spaces from identifier message for {}({})", target, iterator_type);

                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Info,
                    message:
                    "trimming spaces from identifier: `WITH_TRAILING_SPACES `"
                        .to_string(),
                    target: format!("{}({})", target, iterator_type),
                }), "failed to validate trimming trailing spaces from identifier message for {}({})", target, iterator_type);

                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Error,
                    message:
                    "identifier `1STARTING_WITH_NUM` cannot start with number"
                        .to_string(),
                    target: format!("{}({})", target, iterator_type),
                }), "failed to validate identifier starting with number message for {}({})", target, iterator_type);

                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Error,
                    message: "identifier `WITH-INVALID-#.(` contains invalid characters. identifier name can only include [a-zA-Z0-9_] characters.".to_string(),
                    target: format!("{}({})", target, iterator_type),
                }), "failed to validate identifier with invalid chars for {}({})", target, iterator_type);

                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Error,
                    message:
                    "identifier `1INVALID-#. (` is invalid as it contains spaces"
                        .to_string(),
                    target: format!("{}({})", target, iterator_type),
                }), "failed to validate identifier with spaces message for ` 1INVALID-#. ( ` and {}({})", target, iterator_type);

                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Info,
                    message: "trimming spaces from identifier: ` 1INVALID-#. ( `"
                        .to_string(),
                    target: format!("{}({})", target, iterator_type),
                }), "failed to validate trimming spaces environment variable message for ` 1INVALID-#. ( ` and {}({})", target, iterator_type);

                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Info,
                    message: "trimming spaces from identifier: ` 1INVALID-#. ( `"
                        .to_string(),
                    target: format!("{}({})", target, iterator_type),
                }), "failed to validate trimming spaces environment variable message for ` 1INVALID-#. ( ` and {}({})", target, iterator_type);

                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Error,
                    message: "identifier `1INVALID-#. (` cannot start with number"
                        .to_string(),
                    target: format!("{}({})", target, iterator_type),
                }), "failed to validate identifier starting with number for `1INVALID-#. (` and {}({})", target, iterator_type);

                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Error,
                    message: "identifier `1INVALID-#. (` contains invalid characters. identifier name can only include [a-zA-Z0-9_] characters.".to_string(),
                    target: format!("{}({})", target, iterator_type),
                }), "failed to validate identifier with invalid chars for `1INVALID-#. (` and {}({})", target, iterator_type);
            })
        });
    }

    #[serial]
    #[test]
    fn validate_constructors_and_destructors() {
        let test_dir = tempdir().unwrap();

        let paths_root = tempdir().unwrap();
        let paths_bin = paths_root.path().join("bin");
        let paths_usr_bin = paths_root.path().join("usr").join("bin");

        create_dir_all(&paths_bin).unwrap();
        create_dir_all(&paths_usr_bin).unwrap();

        let relative_file = test_dir.path().join("relative_path_with_cwd");
        create_file_with_all_executable_permission(&relative_file);

        let absolute_path = test_dir.path().join("absolute_path");
        create_file_with_all_executable_permission(&absolute_path);

        let absolute_path_not_present = test_dir.path().join("absolute_path_not_present");

        let absolute_path_not_executable = test_dir.path().join("absolute_path_not_executable");
        write(&absolute_path_not_executable, "").unwrap();

        let not_executable = test_dir.path().join("not_executable");
        write(&not_executable, "").unwrap();

        [
            "with_leading_spaces",
            "with_trailing_spaces",
            "valid_command",
        ]
        .iter()
        .for_each(|command| {
            let mut path = paths_bin.clone();
            path.push(command);
            create_file_with_all_executable_permission(&path);
        });
        ["with_relative_path_in_arg", "with_relative_not_present"]
            .iter()
            .for_each(|command| {
                let mut path = paths_usr_bin.clone();
                path.push(command);
                create_file_with_all_executable_permission(&path);
            });

        let mut terrain = Terrain::default();
        let mut biome = Biome::default();

        let command_vec = vec![
            Command::new(
                "not_in_path".to_string(),
                vec![],
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                "with spaces".to_string(),
                vec![],
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                " with_leading_spaces".to_string(),
                vec![],
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                "with_trailing_spaces ".to_string(),
                vec![],
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                "./relative_path_with_cwd".to_string(),
                vec![],
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                "./not_executable".to_string(),
                vec![],
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                "./relative_not_present".to_string(),
                vec![],
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                absolute_path.to_string_lossy().to_string(),
                vec![],
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                absolute_path_not_present.to_string_lossy().to_string(),
                vec![],
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                absolute_path_not_executable.to_string_lossy().to_string(),
                vec![],
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                "with_relative_path_in_arg".to_string(),
                vec!["./present/file".to_string()],
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                "with_relative_arg_not_present".to_string(),
                vec!["./not_present".to_string()],
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                "valid_command".to_string(),
                vec!["some_args1".to_string(), "some_args2".to_string()],
                Some(test_dir.path().to_path_buf()),
            ),
        ];
        let commands = Commands::new(command_vec.clone(), command_vec.clone());

        terrain.terrain_mut().set_constructors(commands.clone());
        biome.set_constructors(commands.clone());
        terrain.terrain_mut().set_destructors(commands.clone());
        biome.set_destructors(commands.clone());

        terrain.update("test_biome".to_string(), biome);

        let real_path = set_env_var(
            "PATH".to_string(),
            Some(format!(
                "{}:{}",
                paths_bin.display(),
                paths_usr_bin.display()
            )),
        );

        let validation_result = terrain
            .validate(test_dir.path())
            .expect_err("to fail")
            .messages;

        let messages: HashSet<ValidationResult> = validation_result.iter().cloned().collect();

        assert_eq!(messages.len(), 96);
        ["none", "test_biome"].iter().for_each(|biome_name| {
            ["constructor", "destructor"]
                .iter()
                .for_each(|operation_type| {
                    ["foreground", "background"]
                        .iter()
                        .for_each(|commands_type| {
                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Error,
                                message: "exe `with spaces` contains whitespaces.".to_string(),
                                target: format!("{}({}:{})", biome_name, operation_type, commands_type),
                            }), "failed to validate whitespace not being present in exe for {}({}:{})", biome_name, operation_type, commands_type);

                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Warn,
                                message: format!("exe `not_in_path` is not present in PATH variable. make sure it is present before {} {} is to be run.", commands_type, operation_type),
                                target: format!("{}({}:{})", biome_name, operation_type, commands_type),
                            }), "failed to validate exe being not in path for {}({}:{})", biome_name, operation_type, commands_type);

                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Warn,
                                message: format!("exe ` with_leading_spaces` has leading / trailing spaces. make sure it is removed {} {} is to be run.", commands_type, operation_type),
                                target: format!("{}({}:{})", biome_name, operation_type, commands_type),
                            }), "failed to validate exe leading spaces for {}({}:{})", biome_name, operation_type, commands_type);

                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Warn,
                                message: format!("exe `with_trailing_spaces ` has leading / trailing spaces. make sure it is removed {} {} is to be run.", commands_type, operation_type),
                                target: format!("{}({}:{})", biome_name, operation_type, commands_type),
                            }), "failed to validate exe trailing for {}({}:{})", biome_name, operation_type, commands_type);

                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Warn,
                                message: format!("exe `./not_executable` does not have permissions to execute. make sure it has correct permissions before {} {} is to be run.", commands_type, operation_type),
                                target: format!("{}({}:{})", biome_name, operation_type, commands_type),
                            }), "failed to validate exe not having execute permission for {}({}:{})", biome_name, operation_type, commands_type);

                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Warn,
                                message: format!("exe `./relative_not_present` is not present in dir: {:?}. make sure it is present before {} {} is to be run.", test_dir.path(), commands_type, operation_type),
                                target: format!("{}({}:{})", biome_name, operation_type, commands_type),
                            }), "failed to validate exe being not in present in relative path for {}({}:{})", biome_name, operation_type, commands_type);

                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Warn,
                                message: format!("exe `{:?}` does not exists. make sure it is present before {} {} is to be run.", absolute_path_not_present, commands_type, operation_type),
                                target: format!("{}({}:{})", biome_name, operation_type, commands_type),
                            }), "failed to validate exe absolute path not being present for {}({}:{})", biome_name, operation_type, commands_type);

                        })
                })
        });

        restore_env_var("PATH".to_string(), real_path);
    }

    fn create_file_with_all_executable_permission(file_path: &PathBuf) {
        write(file_path, "").unwrap();
        let mut perms = metadata(file_path).unwrap().permissions();
        perms.set_mode(perms.mode() | 0o111); // Add executable permission for user, group, and others
        set_permissions(file_path, perms).unwrap();
    }
}
