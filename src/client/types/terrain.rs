use crate::client::args::BiomeArg;
use crate::client::types::biome::Biome;
use crate::client::types::commands::Commands;
use crate::client::types::context::Context;
use crate::client::validation::{
    Target, ValidationFixAction, ValidationMessageLevel, ValidationResults,
};
use crate::common::constants::{
    AUTO_APPLY_ALL, AUTO_APPLY_BACKGROUND, AUTO_APPLY_ENABLED, AUTO_APPLY_OFF, AUTO_APPLY_REPLACE,
    BACKGROUND, BIOMES, CONSTRUCTORS, DESTRUCTORS, EXAMPLE_BIOME, FOREGROUND, NONE, TERRAIN,
};
use crate::common::types::command::Command;
use anyhow::{bail, Context as AnyhowContext, Result};
#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Display;
use std::fs::{read_to_string, write};
use std::path::Path;
use std::str::FromStr;
use toml_edit::DocumentMut;
use tracing::info;

#[cfg_attr(feature = "terrain-schema", derive(JsonSchema))]
#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AutoApply {
    All,
    Background,
    Replace,
    Enabled,
    #[default]
    Off,
}

impl AutoApply {
    pub fn is_background_enabled(&self) -> bool {
        matches!(self, AutoApply::Background) || matches!(self, AutoApply::All)
    }
}

impl Display for AutoApply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match &self {
            AutoApply::All => AUTO_APPLY_ALL.to_string(),
            AutoApply::Background => AUTO_APPLY_BACKGROUND.to_string(),
            AutoApply::Replace => AUTO_APPLY_REPLACE.to_string(),
            AutoApply::Enabled => AUTO_APPLY_ENABLED.to_string(),
            AutoApply::Off => AUTO_APPLY_OFF.to_string(),
        };
        write!(f, "{value}")
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
    pub fn get_validated_and_fixed_terrain(context: &Context) -> Result<(Self, DocumentMut)> {
        let terrain_toml =
            read_to_string(context.toml_path()).context("failed to read terrain.toml")?;
        let toml = terrain_toml
            .parse::<DocumentMut>()
            .context("failed to parse terrain toml")?;
        let unvalidated_terrain = Self::from_toml(terrain_toml)?;
        Self::store_and_get_fixed_terrain(context, unvalidated_terrain, toml)
    }

    pub fn store_and_get_fixed_terrain(
        context: &Context,
        unvalidated_terrain: Terrain,
        terrain_toml: DocumentMut,
    ) -> Result<(Self, DocumentMut)> {
        let validation_results = unvalidated_terrain.validate(context.terrain_dir());
        validation_results.print_validation_message();

        if validation_results
            .results_ref()
            .iter()
            .any(|r| r.level == ValidationMessageLevel::Error)
        {
            bail!("terrain had validation errors");
        }

        if !validation_results.is_fixable() {
            return Ok((unvalidated_terrain, terrain_toml));
        }

        info!("updating the terrain with fixable values");
        let (fixed, fixed_toml) =
            Terrain::fix_invalid_values(&unvalidated_terrain, terrain_toml, validation_results);
        write(context.toml_path(), fixed_toml.to_string())
            .context("failed to write fixed terrain.toml")?;
        Ok((fixed, fixed_toml))
    }

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

    pub fn merged(&self, selected_biome: &BiomeArg) -> Result<Biome> {
        let selected = self.select_biome(selected_biome)?;
        if selected == &self.terrain {
            Ok(self.terrain.clone())
        } else {
            Ok(self.terrain.merge(selected))
        }
    }

    pub(crate) fn select_biome(&self, selected: &BiomeArg) -> Result<&Biome> {
        match selected {
            BiomeArg::None => Ok(&self.terrain),
            BiomeArg::Default => {
                if let Some(default_biome) = &self.default_biome {
                    if let Some(default) = self.biomes.get(default_biome) {
                        Ok(default)
                    } else {
                        bail!("the default biome {:?} does not exists", selected)
                    }
                } else {
                    Ok(&self.terrain)
                }
            }
            BiomeArg::Some(selected) => {
                if let Some(biome) = self.biomes.get(selected) {
                    Ok(biome)
                } else {
                    bail!("the biome {:?} does not exists", selected)
                }
            }
        }
    }

    pub fn validate<'a>(&'a self, terrain_dir: &'a Path) -> ValidationResults<'a> {
        // validate terrain
        let mut results = self.terrain.validate(NONE, terrain_dir);

        // all biomes
        self.biomes.iter().for_each(|(biome_name, biome)| {
            results.append(biome.validate(biome_name, terrain_dir))
        });

        results
    }

    pub fn fix_invalid_values(
        terrain: &Terrain,
        mut toml: DocumentMut,
        validation_results: ValidationResults,
    ) -> (Self, DocumentMut) {
        let mut fixed = terrain.to_owned();
        validation_results
            .results()
            .iter()
            .for_each(|r| match &r.fix_action {
                ValidationFixAction::None => {}
                ValidationFixAction::Trim { biome_name, target } => {
                    let selected = fixed
                        .select_biome(&BiomeArg::from_str(biome_name).unwrap())
                        .unwrap();

                    let mut fixed_biome = selected.to_owned();

                    let biome_toml = if *biome_name == NONE {
                        &mut toml[TERRAIN]
                    } else {
                        &mut toml[BIOMES][biome_name]
                    };

                    match target {
                        Target::Env(e) => {
                            info!(target = r.r#for, "trimming whitespaces from {e}");
                            let trimmed = e.trim();
                            fixed_biome.replace_env_key(e, trimmed);
                            Biome::replace_env_key_toml(biome_toml, e, trimmed);
                        }
                        Target::Alias(a) => {
                            info!(target = r.r#for, "trimming whitespaces from {a}");
                            let trimmed = a.trim();
                            fixed_biome.replace_alias_key(a, trimmed);
                            Biome::replace_alias_key_toml(biome_toml, a, trimmed);
                        }
                        Target::ForegroundConstructor(fc) => {
                            info!(target = r.r#for, "trimming whitespaces from {}", fc.exe());

                            let (idx, command) =
                                fixed_biome.remove_foreground_constructor(fc).unwrap();

                            let fixed = command.trim_exe();

                            Biome::replace_command_exe_toml(
                                biome_toml,
                                CONSTRUCTORS,
                                FOREGROUND,
                                idx,
                                fixed.exe(),
                            );

                            fixed_biome.insert_foreground_constructor(idx, fixed);
                        }
                        Target::BackgroundConstructor(bc) => {
                            info!(target = r.r#for, "trimming whitespaces from {}", bc.exe());

                            let (idx, command) =
                                fixed_biome.remove_background_constructor(bc).unwrap();
                            let fixed = command.trim_exe();

                            Biome::replace_command_exe_toml(
                                biome_toml,
                                CONSTRUCTORS,
                                BACKGROUND,
                                idx,
                                fixed.exe(),
                            );

                            fixed_biome.insert_background_constructor(idx, fixed);
                        }
                        Target::ForegroundDestructor(fd) => {
                            info!(target = r.r#for, "trimming whitespaces from {}", fd.exe());

                            let (idx, command) =
                                fixed_biome.remove_foreground_destructor(fd).unwrap();
                            let fixed = command.trim_exe();

                            Biome::replace_command_exe_toml(
                                biome_toml,
                                DESTRUCTORS,
                                FOREGROUND,
                                idx,
                                fixed.exe(),
                            );

                            fixed_biome.insert_foreground_destructor(idx, fixed);
                        }
                        Target::BackgroundDestructor(bd) => {
                            info!(target = r.r#for, "trimming whitespaces from {}", bd.exe());
                            let (idx, command) =
                                fixed_biome.remove_background_destructor(bd).unwrap();
                            let fixed = command.trim_exe();

                            Biome::replace_command_exe_toml(
                                biome_toml,
                                DESTRUCTORS,
                                BACKGROUND,
                                idx,
                                fixed.exe(),
                            );
                            fixed_biome.insert_background_destructor(idx, fixed);
                        }
                    }
                    fixed.update(biome_name.to_string(), fixed_biome);
                }
            });
        (fixed, toml)
    }

    pub fn from_toml(toml_str: String) -> Result<Self> {
        let mut terrain: Self =
            toml::from_str(&toml_str).context("failed to parse terrain from toml")?;

        terrain.terrain.set_name(NONE.to_string());
        terrain.biomes.iter_mut().for_each(|(name, biome)| {
            biome.set_name(name.to_string());
        });

        Ok(terrain)
    }

    pub fn to_toml(&self, terrain_dir: &Path) -> Result<String> {
        let result = self.validate(terrain_dir);
        if result
            .results()
            .iter()
            .any(|r| r.level == ValidationMessageLevel::Error)
        {
            bail!("failed to write terrain as it had validation errors");
        }

        toml::to_string(&self).context("failed to convert terrain to toml")
    }

    pub(crate) fn update(&mut self, biome_name: String, updated: Biome) {
        if biome_name == NONE {
            self.terrain = updated
        } else {
            self.biomes.insert(biome_name, updated);
        }
    }

    pub fn example() -> Self {
        let terrain = Biome::example(NONE.to_string());

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
                None,
            )],
            vec![Command::new(
                "/bin/bash".to_string(),
                vec![
                    "-c".to_string(),
                    "${PWD}/tests/scripts/print_num_for_10_sec".to_string(),
                ],
                None,
                None,
            )],
        );

        let biome_destructors: Commands = Commands::new(
            vec![Command::new(
                "/bin/echo".to_string(),
                vec!["exiting biome example_biome".to_string()],
                None,
                None,
            )],
            vec![Command::new(
                "/bin/bash".to_string(),
                vec![
                    "-c".to_string(),
                    "${TERRAIN_DIR}/tests/scripts/print_num_for_10_sec".to_string(),
                ],
                None,
                None,
            )],
        );

        let example_biome = Biome::new(
            EXAMPLE_BIOME.to_string(),
            biome_envs,
            biome_aliases,
            biome_constructors,
            biome_destructors,
        );

        let mut biomes: BTreeMap<String, Biome> = BTreeMap::new();
        biomes.insert(EXAMPLE_BIOME.to_string(), example_biome);

        Terrain::new(
            terrain,
            biomes,
            Some(EXAMPLE_BIOME.to_string()),
            AutoApply::Off,
        )
    }
}

impl Default for Terrain {
    fn default() -> Self {
        let mut terrain = Biome::default();
        terrain.set_name(NONE.to_string());
        Terrain::new(terrain, BTreeMap::new(), None, AutoApply::default())
    }
}

#[cfg(test)]
pub mod tests {
    use crate::client::test_utils::constants::{
        WITH_EXAMPLE_TERRAIN_TOML_COMMENTS, WITH_EXAMPLE_TERRAIN_TOML_COMMENTS_SPACES,
    };
    use crate::client::test_utils::{restore_env_var, set_env_var};
    use crate::client::types::biome::Biome;
    use crate::client::types::commands::Commands;
    use crate::client::types::context::Context;
    use crate::client::types::terrain::{AutoApply, Terrain};
    use crate::client::validation::{
        Target, ValidationFixAction, ValidationMessageLevel, ValidationResult,
    };
    use crate::common::constants::{NONE, TERRAIN_TOML};
    use crate::common::execute::MockExecutor;
    use crate::common::types::command::{Command, CommandsType};
    use serial_test::serial;
    use std::collections::BTreeMap;
    use std::fs::{copy, create_dir_all, metadata, read_to_string, set_permissions, write};
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::path::PathBuf;
    use std::str::FromStr;
    use tempfile::tempdir;
    use toml_edit::DocumentMut;

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
            None,
        )];
        let biome_constructor_background: Vec<Command> = vec![];
        let biome_destructor_foreground: Vec<Command> = vec![Command::new(
            "/bin/echo".to_string(),
            vec!["exiting biome ".to_string() + &name],
            None,
            None,
        )];
        let biome_destructor_background: Vec<Command> = vec![];

        let biome_constructor =
            Commands::new(biome_constructor_foreground, biome_constructor_background);
        let biome_destructor =
            Commands::new(biome_destructor_foreground, biome_destructor_background);

        Biome::new(
            name,
            biome_envs,
            biome_aliases,
            biome_constructor,
            biome_destructor,
        )
    }

    #[cfg(test)]
    pub(crate) fn set_auto_apply(terrain: &mut Terrain, auto_apply: &str) {
        terrain.auto_apply = AutoApply::from_str(auto_apply).unwrap();
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

        let path = PathBuf::new();
        let messages = terrain.validate(&path).results();

        assert_eq!(messages.len(), 44);

        [NONE, "test_biome"].iter().for_each(|biome_name| {
            ["env", "alias"].iter().for_each(|identifier_type| {
                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Error,
                    message: "empty identifier is not allowed".to_string(),
                    r#for: format!("{biome_name}({identifier_type})"),
                    fix_action: ValidationFixAction::None,
                }), "failed to validate empty identifier message for {biome_name}({identifier_type})");

                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Error,
                    message:
                    "identifier 'TEST WITH SPACES' is invalid as it contains spaces"
                        .to_string(),
                    r#for: format!("{biome_name}({identifier_type})"),
                    fix_action: ValidationFixAction::None,
                }), "failed to validate identifier with spaces message for {biome_name}({identifier_type})");

                let fix_action = if identifier_type == &"env" {
                    ValidationFixAction::Trim { biome_name, target: Target::Env(" WITH_LEADING_SPACES") }
                } else {
                    ValidationFixAction::Trim { biome_name, target: Target::Alias(" WITH_LEADING_SPACES") }
                };
                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Warn,
                    message:
                    "trimming spaces from identifier: ' WITH_LEADING_SPACES'"
                        .to_string(),
                    r#for: format!("{biome_name}({identifier_type})"),
                    fix_action,
                }), "failed to validate trimming leading spaces from identifier message for {biome_name}({identifier_type})");

                let fix_action = if identifier_type == &"env" {
                    ValidationFixAction::Trim { biome_name, target: Target::Env("WITH_TRAILING_SPACES ") }
                } else {
                    ValidationFixAction::Trim { biome_name, target: Target::Alias("WITH_TRAILING_SPACES ") }
                };
                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Warn,
                    message:
                    "trimming spaces from identifier: 'WITH_TRAILING_SPACES '"
                        .to_string(),
                    r#for: format!("{biome_name}({identifier_type})"),
                    fix_action,
                }), "failed to validate trimming trailing spaces from identifier message for {biome_name}({identifier_type})");

                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Error,
                    message:
                    "identifier '1STARTING_WITH_NUM' cannot start with number"
                        .to_string(),
                    r#for: format!("{biome_name}({identifier_type})"),
                    fix_action: ValidationFixAction::None,
                }), "failed to validate identifier starting with number message for {biome_name}({identifier_type})");

                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Error,
                    message: "identifier 'WITH-INVALID-#.(' contains invalid characters. identifier name can only include [a-zA-Z0-9_] characters.".to_string(),
                    r#for: format!("{biome_name}({identifier_type})"),
                    fix_action: ValidationFixAction::None,
                }), "failed to validate identifier with invalid chars for {biome_name}({identifier_type})");

                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Error,
                    message:
                    "identifier '1INVALID-#. (' is invalid as it contains spaces"
                        .to_string(),
                    r#for: format!("{biome_name}({identifier_type})"),
                    fix_action: ValidationFixAction::None,
                }), "failed to validate identifier with spaces message for ' 1INVALID-#. ( ' and {biome_name}({identifier_type})");

                let fix_action = if identifier_type == &"env" {
                    ValidationFixAction::Trim { biome_name, target: Target::Env(" 1INVALID-#. ( ") }
                } else {
                    ValidationFixAction::Trim { biome_name, target: Target::Alias(" 1INVALID-#. ( ") }
                };
                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Warn,
                    message: "trimming spaces from identifier: ' 1INVALID-#. ( '"
                        .to_string(),
                    r#for: format!("{biome_name}({identifier_type})"),
                    fix_action: fix_action.clone(),
                }), "failed to validate trimming spaces environment variable message for ' 1INVALID-#. ( ' and {biome_name}({identifier_type})");

                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Warn,
                    message: "trimming spaces from identifier: ' 1INVALID-#. ( '"
                        .to_string(),
                    r#for: format!("{biome_name}({identifier_type})"),
                    fix_action,
                }), "failed to validate trimming spaces environment variable message for ' 1INVALID-#. ( ' and {biome_name}({identifier_type})");

                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Error,
                    message: "identifier '1INVALID-#. (' cannot start with number"
                        .to_string(),
                    r#for: format!("{biome_name}({identifier_type})"),
                    fix_action: ValidationFixAction::None,
                }), "failed to validate identifier starting with number for '1INVALID-#. (' and {biome_name}({identifier_type})");

                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Error,
                    message: "identifier '1INVALID-#. (' contains invalid characters. identifier name can only include [a-zA-Z0-9_] characters.".to_string(),
                    r#for: format!("{biome_name}({identifier_type})"),
                    fix_action: ValidationFixAction::None,
                }), "failed to validate identifier with invalid chars for '1INVALID-#. (' and {biome_name}({identifier_type})");
            })
        });
    }

    #[serial]
    #[test]
    fn validate_constructors_and_destructors() {
        let test_dir = tempdir().unwrap();
        let relative_dir = test_dir.path().join("relative_dir");
        create_dir_all(&relative_dir).unwrap();

        let symlink_cwd = test_dir.path().join("symlink_cwd");
        symlink(&relative_dir, &symlink_cwd).unwrap();

        let paths_root = tempdir().unwrap();
        let paths_bin = paths_root.path().join("bin");
        let paths_usr_bin = paths_root.path().join("usr").join("bin");

        create_dir_all(&paths_bin).unwrap();
        create_dir_all(&paths_usr_bin).unwrap();

        let relative_file = test_dir.path().join("relative_path_with_cwd");
        create_file_with_all_executable_permission(&relative_file);

        let symlink_exe = test_dir.path().join("symlink_exe");
        symlink(&relative_file, &symlink_exe).unwrap();

        let symlink_symlink_exe = test_dir.path().join("symlink_symlink_exe");
        symlink(&symlink_exe, &symlink_symlink_exe).unwrap();

        let symlink_file = test_dir.path().join("symlink_file");
        symlink(&relative_file, &symlink_file).unwrap();

        let absolute_exe_path = test_dir.path().join("absolute_path");
        create_file_with_all_executable_permission(&absolute_exe_path);

        let absolute_path_not_present = test_dir.path().join("absolute_path_not_present");

        let absolute_path_not_executable = test_dir.path().join("absolute_path_not_executable");
        write(&absolute_path_not_executable, "").unwrap();

        let not_executable = test_dir.path().join("not_executable");
        write(&not_executable, "").unwrap();

        let symlink_not_executable = test_dir.path().join("symlink_not_executable");
        symlink(&not_executable, &symlink_not_executable).unwrap();

        let symlink_symlink_not_executable = test_dir.path().join("symlink_symlink_not_executable");
        symlink(&symlink_not_executable, &symlink_symlink_not_executable).unwrap();

        [
            "sudo",
            "with_leading_spaces",
            "with_trailing_spaces",
            "valid_command",
        ]
        .iter()
        .for_each(|command| {
            let path = paths_bin.join(command);
            create_file_with_all_executable_permission(&path);
        });
        ["with_relative_path_in_arg", "with_relative_not_present"]
            .iter()
            .for_each(|command| {
                let path = paths_usr_bin.join(command);
                create_file_with_all_executable_permission(&path);
            });

        let mut terrain = Terrain::default();
        let mut biome = Biome::default();

        let leading_space_command = Command::new(
            " with_leading_spaces".to_string(),
            vec![],
            None,
            Some(test_dir.path().to_path_buf()),
        );
        let trailing_space_command = Command::new(
            "with_trailing_spaces ".to_string(),
            vec![],
            None,
            Some(test_dir.path().to_path_buf()),
        );
        let command_vec = vec![
            leading_space_command.clone(),
            trailing_space_command.clone(),
            Command::new("".to_string(), vec![], None, None),
            Command::new(
                "not_in_path".to_string(),
                vec![],
                None,
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                "with spaces".to_string(),
                vec![],
                None,
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                "./relative_path_with_cwd".to_string(),
                vec![],
                None,
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                "./not_executable".to_string(),
                vec![],
                None,
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                "./relative_not_present".to_string(),
                vec![],
                None,
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                "./relative_not_present_current_dir".to_string(),
                vec![],
                None,
                None,
            ),
            Command::new(
                absolute_exe_path.to_string_lossy().to_string(),
                vec![],
                None,
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                absolute_path_not_present.to_string_lossy().to_string(),
                vec![],
                None,
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                absolute_path_not_executable.to_string_lossy().to_string(),
                vec![],
                None,
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                "with_relative_arg_not_present".to_string(),
                vec!["./not_present".to_string()],
                None,
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                symlink_symlink_exe.to_string_lossy().to_string(),
                vec![],
                None,
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                "./symlink_symlink_not_executable".to_string(),
                vec![],
                None,
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                "valid_command".to_string(),
                vec!["some_args1".to_string(), "some_args2".to_string()],
                None,
                Some(paths_usr_bin.clone()),
            ),
            Command::new(
                "valid_command".to_string(),
                vec!["some_args1".to_string(), "some_args2".to_string()],
                None,
                Some(PathBuf::from("./relative_dir")),
            ),
            Command::new(
                "valid_command".to_string(),
                vec!["some_args1".to_string(), "some_args2".to_string()],
                None,
                Some(PathBuf::from("${RELATIVE_DIR}")),
            ),
            Command::new(
                "valid_command".to_string(),
                vec!["some_args1".to_string(), "some_args2".to_string()],
                None,
                Some(PathBuf::from("./relative_dir_does_not_exist")),
            ),
            Command::new(
                "valid_command".to_string(),
                vec!["some_args1".to_string(), "some_args2".to_string()],
                None,
                Some(PathBuf::from("/absolute_dir_does_not_exist")),
            ),
            Command::new(
                "valid_command".to_string(),
                vec!["some_args1".to_string(), "some_args2".to_string()],
                None,
                Some(absolute_exe_path.clone()),
            ),
            Command::new(
                "valid_command".to_string(),
                vec!["some_args1".to_string(), "some_args2".to_string()],
                None,
                Some(PathBuf::from("./relative_path_with_cwd")),
            ),
            Command::new(
                "sudo".to_string(),
                vec!["whoami".to_string()],
                None,
                Some(test_dir.path().to_path_buf()),
            ),
            Command::new(
                "valid_command".to_string(),
                vec!["some_args1".to_string(), "some_args2".to_string()],
                None,
                Some(symlink_file.clone()),
            ),
            Command::new(
                "valid_command".to_string(),
                vec!["some_args1".to_string(), "some_args2".to_string()],
                None,
                Some(symlink_cwd.clone()),
            ),
        ];
        let commands = Commands::new(command_vec.clone(), command_vec);

        terrain
            .terrain_mut()
            .add_envs(vec![("RELATIVE_DIR", "relative_dir")]);
        terrain.terrain_mut().set_constructors(commands.clone());
        terrain.terrain_mut().set_destructors(commands.clone());

        biome.add_envs(vec![("RELATIVE_DIR", "relative_dir")]);
        biome.set_constructors(commands.clone());
        biome.set_destructors(commands);
        terrain.update("test_biome".to_string(), biome);

        let real_path = set_env_var(
            "PATH",
            Some(&format!(
                "{}:{}",
                paths_bin.display(),
                paths_usr_bin.display()
            )),
        );

        let messages = terrain.validate(test_dir.path()).results();

        assert_eq!(messages.len(), 160);
        [NONE, "test_biome"].iter().for_each(|biome_name| {
            ["constructor", "destructor"]
                .iter()
                .for_each(|operation_type| {
                    ["foreground", "background"]
                        .iter()
                        .for_each(|commands_type| {
                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Error,
                                message: format!(
                                    "exe cannot be empty, make sure it is set before {commands_type} {operation_type} is to be run.",
                                ),
                                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                                fix_action: ValidationFixAction::None,
                            }), "failed to validate empty exe for {biome_name}({operation_type}:{commands_type})");

                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Error,
                                message: "exe 'with spaces' contains whitespaces.".to_string(),
                                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                                fix_action: ValidationFixAction::None,
                            }), "failed to validate whitespace not being present in exe for {biome_name}({operation_type}:{commands_type})");

                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Error,
                                message: format!("exe 'not_in_path' is not present in PATH variable. make sure it is present before {commands_type} {operation_type} is to be run."),
                                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                                fix_action: ValidationFixAction::None,
                            }), "failed to validate exe being not in path for {biome_name}({operation_type}:{commands_type})");

                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Error,
                                message: format!(
                                    "cwd: '{}' does not exists for command exe: 'valid_command' args: 'some_args1 some_args2'.",
                                    test_dir.path().join("./relative_dir_does_not_exist").display(),
                                ),
                                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                                fix_action: ValidationFixAction::None,
                            }), "failed to validate relative cwd does not exist for {biome_name}({operation_type}:{commands_type})");

                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Error,
                                message: "cwd: '/absolute_dir_does_not_exist' does not exists for command exe: 'valid_command' args: 'some_args1 some_args2'.".to_string(),
                                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                                fix_action: ValidationFixAction::None,
                            }), "failed to validate absolute cwd does not exist for {biome_name}({operation_type}:{commands_type})");

                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Error,
                                message: format!(
                                    "cwd: '{}' is not a directory for command exe: 'valid_command' args: 'some_args1 some_args2'.",
                                    absolute_exe_path.display(),
                                ),
                                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                                fix_action: ValidationFixAction::None,
                            }), "failed to validate absolute cwd does not exist for {biome_name}({operation_type}:{commands_type})");

                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Error,
                                message: format!("cwd: '{}' is not a directory for command exe: 'valid_command' args: 'some_args1 some_args2'.",
                                                 test_dir.path().join("./relative_path_with_cwd").display()),
                                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                                fix_action: ValidationFixAction::None,
                            }), "failed to validate cwd not a directory exist for {biome_name}({operation_type}:{commands_type})");

                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Error,
                                message: format!(
                                    "cwd: '{}' is a symlink but does not resolve to directory ({}) for command exe: 'valid_command' args: 'some_args1 some_args2'.",
                                    symlink_file.display(),
                                    relative_file.display(),
                                ),
                                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                                fix_action: ValidationFixAction::None,
                            }), "failed to validate symlink cwd file for {biome_name}({operation_type}:{commands_type})");

                            let fix_action = get_test_fix_action(&leading_space_command, biome_name, operation_type, commands_type);
                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Warn,
                                message: format!("exe ' with_leading_spaces' has leading / trailing spaces. make sure it is removed before {commands_type} {operation_type} is to be run."),
                                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                                fix_action,
                            }), "failed to validate exe leading spaces for {biome_name}({operation_type}:{commands_type})");

                            let fix_action = get_test_fix_action(&trailing_space_command, biome_name, operation_type, commands_type);
                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Warn,
                                message: format!("exe 'with_trailing_spaces ' has leading / trailing spaces. make sure it is removed before {commands_type} {operation_type} is to be run."),
                                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                                fix_action,
                            }), "failed to validate exe trailing for {biome_name}({operation_type}:{commands_type})");

                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Error,
                                message: format!("exe './not_executable' does not have permissions to execute. make sure it has correct permissions before {commands_type} {operation_type} is to be run."),
                                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                                fix_action: ValidationFixAction::None,
                            }), "failed to validate exe not having execute permission for {biome_name}({operation_type}:{commands_type})");

                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Error,
                                message: format!("exe './symlink_symlink_not_executable' does not have permissions to execute. make sure it has correct permissions before {commands_type} {operation_type} is to be run."),
                                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                                fix_action: ValidationFixAction::None,
                            }), "failed to validate symlink exe not having execute permission for {biome_name}({operation_type}:{commands_type})");

                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Error,
                                message: format!("exe './relative_not_present' does not exists. make sure it is present before {commands_type} {operation_type} is to be run."),
                                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                                fix_action: ValidationFixAction::None,
                            }), "failed to validate exe being not in present in relative path for {biome_name}({operation_type}:{commands_type})");

                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Error,
                                message: format!("exe './relative_not_present_current_dir' does not exists. make sure it is present before {commands_type} {operation_type} is to be run."),
                                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                                fix_action: ValidationFixAction::None,
                            }), "failed to validate exe being not in present in relative path for terrain directory {biome_name}({operation_type}:{commands_type})");

                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Error,
                                message: format!("exe '{}' does not exists. make sure it is present before {commands_type} {operation_type} is to be run.", absolute_path_not_present.display()),
                                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                                fix_action: ValidationFixAction::None,
                            }), "failed to validate exe absolute path not being present for {biome_name}({operation_type}:{commands_type})");

                            if commands_type == &CommandsType::Background.to_string() {
                                assert!(messages.contains(&ValidationResult {
                                    level: ValidationMessageLevel::Warn,
                                    message: "command exe: 'sudo' args: 'whoami' uses sudo. Running sudo commands in background is not allowed (see terrainium docs for more info).".to_string(),
                                    r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                                    fix_action: ValidationFixAction::None,
                                }), "failed to validate exe containing sudo for {biome_name}({operation_type}:{commands_type})");
                            } else {
                                assert!(messages.contains(&ValidationResult {
                                    level: ValidationMessageLevel::Warn,
                                    message: "command exe: 'sudo' args: 'whoami' uses sudo. Running sudo commands in foreground will block entering / exiting shell till user is authenticated.".to_string(),
                                    r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                                    fix_action: ValidationFixAction::None,
                                }), "failed to validate exe containing sudo for {biome_name}({operation_type}:{commands_type})");
                            }

                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Info,
                                message: format!(
                                    "cwd: '{}/${{RELATIVE_DIR}}' contains environment variable references: 'RELATIVE_DIR' for exe: 'valid_command' args: 'some_args1 some_args2'. Make sure they are set before the {commands_type} {operation_type} is executed",
                                    test_dir.path().display(),
                                ),
                                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                                fix_action: ValidationFixAction::None,
                            }), "failed to validate cwd with env var for {biome_name}({operation_type}:{commands_type})");
                        })
                })
        });

        restore_env_var("PATH", real_path);
    }

    #[serial]
    #[test]
    fn test_validation_fix_trim() {
        let test_dir = tempdir().unwrap();

        let paths_root = tempdir().unwrap();
        let paths_bin = paths_root.path().join("bin");

        create_dir_all(&paths_bin).unwrap();

        let mut map = BTreeMap::new();
        map.insert(
            " WITH_LEADING_SPACES".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );
        map.insert(
            "WITH_TRAILING_SPACES ".to_string(),
            "VALUE_WITHOUT_SPACES".to_string(),
        );

        let leading_space_command = Command::new(
            " with_leading_spaces".to_string(),
            vec![],
            None,
            Some(test_dir.path().to_path_buf()),
        );
        let trailing_space_command = Command::new(
            "with_trailing_spaces ".to_string(),
            vec![],
            None,
            Some(test_dir.path().to_path_buf()),
        );

        ["with_leading_spaces", "with_trailing_spaces"]
            .iter()
            .for_each(|command| {
                let mut path = paths_bin.clone();
                path.push(command);
                create_file_with_all_executable_permission(&path);
            });

        let command_vec = vec![
            leading_space_command.clone(),
            trailing_space_command.clone(),
        ];
        let commands = Commands::new(command_vec.clone(), command_vec);

        let mut terrain = Terrain::default();
        let mut biome = Biome::default();
        terrain.terrain_mut().set_envs(map.clone());
        terrain.terrain_mut().set_aliases(map.clone());
        terrain.terrain_mut().set_constructors(commands.clone());
        terrain.terrain_mut().set_destructors(commands.clone());
        biome.set_envs(map.clone());
        biome.set_aliases(map);
        biome.set_constructors(commands.clone());
        biome.set_destructors(commands);

        terrain.update("test_biome".to_string(), biome);

        let before = terrain.clone();

        let real_path = set_env_var(
            "PATH",
            Some(&format!(
                "{}:{}",
                paths_root.path().display(),
                paths_bin.display(),
            )),
        );
        let messages = before.validate(test_dir.path()).results();

        assert_eq!(messages.len(), 24);
        [NONE, "test_biome"].iter().for_each(|biome_name| {
            ["env", "alias"].iter().for_each(|identifier_type| {
                let fix_action = if identifier_type == &"env" {
                    ValidationFixAction::Trim { biome_name, target: Target::Env(" WITH_LEADING_SPACES") }
                } else {
                    ValidationFixAction::Trim { biome_name, target: Target::Alias(" WITH_LEADING_SPACES") }
                };
                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Warn,
                    message:
                    "trimming spaces from identifier: ' WITH_LEADING_SPACES'"
                        .to_string(),
                    r#for: format!("{biome_name}({identifier_type})"),
                    fix_action,
                }), "failed to validate trimming leading spaces from identifier message for {biome_name}({identifier_type})");

                let fix_action = if identifier_type == &"env" {
                    ValidationFixAction::Trim { biome_name, target: Target::Env("WITH_TRAILING_SPACES ") }
                } else {
                    ValidationFixAction::Trim { biome_name, target: Target::Alias("WITH_TRAILING_SPACES ") }
                };
                assert!(messages.contains(&ValidationResult {
                    level: ValidationMessageLevel::Warn,
                    message:
                    "trimming spaces from identifier: 'WITH_TRAILING_SPACES '"
                        .to_string(),
                    r#for: format!("{biome_name}({identifier_type})"),
                    fix_action,
                }), "failed to validate trimming trailing spaces from identifier message for {biome_name}({identifier_type})");
            });

            ["constructor", "destructor"]
                .iter()
                .for_each(|operation_type| {
                    ["foreground", "background"]
                        .iter()
                        .for_each(|commands_type| {
                            let fix_action = get_test_fix_action(&leading_space_command, biome_name, operation_type, commands_type);
                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Warn,
                                message: format!("exe ' with_leading_spaces' has leading / trailing spaces. make sure it is removed before {commands_type} {operation_type} is to be run."),
                                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                                fix_action,
                            }), "failed to validate exe leading spaces for {biome_name}({operation_type}:{commands_type})");

                            let fix_action = get_test_fix_action(&trailing_space_command, biome_name, operation_type, commands_type);
                            assert!(messages.contains(&ValidationResult {
                                level: ValidationMessageLevel::Warn,
                                message: format!("exe 'with_trailing_spaces ' has leading / trailing spaces. make sure it is removed before {commands_type} {operation_type} is to be run."),
                                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                                fix_action,
                            }), "failed to validate exe trailing for {biome_name}({operation_type}:{commands_type})");
                        })
                })
        });

        let toml = terrain
            .to_toml(test_dir.path())
            .unwrap()
            .parse::<DocumentMut>()
            .unwrap();

        let (fixed, _) =
            Terrain::fix_invalid_values(&terrain, toml, before.validate(test_dir.path()));
        let fixed_result = fixed.validate(test_dir.path());

        assert!(fixed_result.results().is_empty());

        restore_env_var("PATH", real_path);
    }

    #[test]
    fn fix_invalid_values() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join(TERRAIN_TOML);
        copy(WITH_EXAMPLE_TERRAIN_TOML_COMMENTS_SPACES, &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let context = Context::build(
            current_dir.path(),
            central_dir.path(),
            false,
            MockExecutor::new(),
        );

        Terrain::get_validated_and_fixed_terrain(&context).expect("terrain to fixed");

        let actual = read_to_string(terrain_toml).unwrap();
        let expected = read_to_string(WITH_EXAMPLE_TERRAIN_TOML_COMMENTS).unwrap();
        assert_eq!(expected, actual);
    }

    fn get_test_fix_action<'a>(
        command: &'a Command,
        biome_name: &&'a str,
        operation_type: &&str,
        commands_type: &&str,
    ) -> ValidationFixAction<'a> {
        let fix_action = if operation_type == &"constructor" {
            if commands_type == &"foreground" {
                ValidationFixAction::Trim {
                    biome_name,
                    target: Target::ForegroundConstructor(command),
                }
            } else {
                ValidationFixAction::Trim {
                    biome_name,
                    target: Target::BackgroundConstructor(command),
                }
            }
        } else if commands_type == &"foreground" {
            ValidationFixAction::Trim {
                biome_name,
                target: Target::ForegroundDestructor(command),
            }
        } else {
            ValidationFixAction::Trim {
                biome_name,
                target: Target::BackgroundDestructor(command),
            }
        };
        fix_action
    }

    fn create_file_with_all_executable_permission(file_path: &PathBuf) {
        write(file_path, "").unwrap();
        let mut perms = metadata(file_path).unwrap().permissions();
        perms.set_mode(perms.mode() | 0o111); // Add executable permission for user, group, and others
        set_permissions(file_path, perms).unwrap();
    }
}
