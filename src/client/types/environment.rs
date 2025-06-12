use crate::client::args::BiomeArg;
use crate::client::types::biome::Biome;
use crate::client::types::commands::Commands;
use crate::client::types::terrain::{AutoApply, Terrain};
use crate::client::validation::{
    ValidationError, ValidationFixAction, ValidationMessageLevel, ValidationResult,
    ValidationResults,
};
use crate::common::constants::{TERRAIN_DIR, TERRAIN_SELECTED_BIOME};
use anyhow::{bail, Context, Result};
use handlebars::Handlebars;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

#[derive(Serialize, Debug, PartialEq)]
pub struct Environment {
    name: String,
    default_biome: Option<String>,
    selected_biome: String,
    auto_apply: AutoApply,
    merged: Biome,
}

impl Environment {
    pub fn from(terrain: &Terrain, selected_biome: BiomeArg, terrain_dir: &Path) -> Result<Self> {
        let mut merged: Biome = terrain.merged(&selected_biome)?;
        // add required envs
        merged.insert_env(
            TERRAIN_DIR.to_string(),
            terrain_dir.to_string_lossy().to_string(),
        );
        merged.insert_env(TERRAIN_SELECTED_BIOME.to_string(), merged.name());

        merged.substitute_envs();
        merged
            .substitute_cwd(terrain_dir)
            .context("failed to substitute cwd for environment")?;

        let environment = Environment {
            name: terrain.name().clone(),
            default_biome: terrain.default_biome().clone(),
            selected_biome: merged.name(),
            auto_apply: terrain.auto_apply().clone(),
            merged,
        };
        let result = environment.validate();
        if let Err(e) = &result {
            e.results.print_validation_message();
            bail!("failed to validate environment");
        }
        result.unwrap().print_validation_message();

        Ok(environment)
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn default_biome(&self) -> &Option<String> {
        &self.default_biome
    }

    pub fn auto_apply(&self) -> &AutoApply {
        &self.auto_apply
    }

    pub fn selected_biome(&self) -> &String {
        &self.selected_biome
    }

    pub fn merged(&self) -> &Biome {
        &self.merged
    }

    pub fn envs(&self) -> BTreeMap<String, String> {
        self.merged.envs().clone()
    }

    pub fn aliases(&self) -> BTreeMap<String, String> {
        self.merged.aliases().clone()
    }

    pub fn constructors(&self) -> Commands {
        self.merged.constructors().clone()
    }

    pub fn destructors(&self) -> Commands {
        self.merged.destructors().clone()
    }

    pub(crate) fn insert_env(&mut self, key: String, value: String) {
        self.merged.insert_env(key, value);
    }

    pub(crate) fn append_envs(&mut self, envs: BTreeMap<String, String>) {
        self.merged.append_envs(envs);
    }

    pub(crate) fn to_rendered(
        &self,
        main_template: String,
        templates: BTreeMap<String, String>,
    ) -> Result<String> {
        render(main_template, templates, self)
    }

    fn validate_envs(&self) -> ValidationResults {
        let mut result = HashSet::new();
        self.merged.envs().iter().for_each(|(k, v)| {
            // validate if all env references are resolved
            let env_refs = Biome::get_envs_to_substitute(v);
            if !env_refs.is_empty() {
                let refs = env_refs.join("', '");
                result.insert(ValidationResult {
                    level: ValidationMessageLevel::Warn,
                    message: format!(
                        "environment variable '{k}' contains reference to variables \
                     ('{refs}') that are not defined in terrain.toml and system environment variables. \
                      ensure that variables ('{refs}') are set before using '{k}' environment variable."
                    ),
                    r#for: self.selected_biome().clone(),
                    fix_action: ValidationFixAction::None,
                });
            }
        });
        ValidationResults::new(false, result)
    }

    pub(crate) fn validate(&self) -> std::result::Result<ValidationResults, ValidationError> {
        let results = self.validate_envs();
        if results
            .results_ref()
            .iter()
            .any(|val| val.level == ValidationMessageLevel::Error)
        {
            return Err(ValidationError { results });
        }

        Ok(results)
    }

    #[cfg(test)]
    pub fn build(default_biome: Option<String>, selected_biome: String, merged: &Biome) -> Self {
        Environment {
            name: "terrainium".to_string(),
            default_biome,
            selected_biome,
            auto_apply: AutoApply::default(),
            merged: merged.clone(),
        }
    }

    #[cfg(test)]
    pub fn merged_mut(&mut self) -> &mut Biome {
        &mut self.merged
    }
}

pub(crate) fn render<T: Serialize>(
    main_template: String,
    templates: BTreeMap<String, String>,
    arg: T,
) -> Result<String> {
    let mut handlebars = Handlebars::new();
    templates.iter().for_each(|(name, template)| {
        handlebars
            .register_template_string(name, template)
            .expect("failed to register template")
    });

    handlebars
        .render(&main_template, &arg)
        .context("failed to render template ".to_string() + &main_template)
}

#[cfg(test)]
mod tests {
    use crate::client::args::BiomeArg;
    use crate::client::shell::{Shell, Zsh};
    use crate::client::test_utils::{
        expected_aliases_example_biome, expected_constructor_background_example_biome,
        expected_constructor_foreground_example_biome, expected_constructors_example_biome,
        expected_destructor_background_example_biome, expected_destructor_foreground_example_biome,
        expected_destructors_example_biome, expected_env_vars_example_biome, restore_env_var,
        set_env_var,
    };
    use crate::client::types::biome::Biome;
    use crate::client::types::commands::Commands;
    use crate::client::types::environment::Environment;
    use crate::client::types::terrain::tests::{
        add_biome, force_set_invalid_default_biome, get_test_biome,
    };
    use crate::client::types::terrain::Terrain;
    use crate::client::validation::{
        ValidationFixAction, ValidationMessageLevel, ValidationResult,
    };
    use crate::common::constants::{EXAMPLE_BIOME, NONE};
    use crate::common::types::command::Command;
    use anyhow::Result;
    use std::collections::BTreeMap;
    use std::fs;
    use std::fs::create_dir_all;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn environment_from_empty_terrain() -> Result<()> {
        let mut terrain = Terrain::default();
        terrain
            .terrain_mut()
            .insert_env("TERRAIN_DIR".to_string(), "".to_string());
        terrain
            .terrain_mut()
            .insert_env("TERRAIN_SELECTED_BIOME".to_string(), NONE.to_string());

        let expected: Environment = Environment::build(None, NONE.to_string(), terrain.terrain());

        let actual = Environment::from(&Terrain::default(), BiomeArg::Default, &PathBuf::new())
            .expect("no error to be thrown");

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn environment_from_example_but_no_default_or_selected() -> Result<()> {
        let terrain_dir = tempdir()?;
        let mut terrain = Terrain::example();
        terrain
            .terrain_mut()
            .insert_env("TERRAIN_DIR".to_string(), "".to_string());
        terrain
            .terrain_mut()
            .insert_env("TERRAIN_SELECTED_BIOME".to_string(), NONE.to_string());

        force_set_invalid_default_biome(&mut terrain, None);
        terrain.terrain_mut().substitute_envs();
        terrain.terrain_mut().substitute_cwd(terrain_dir.path())?;

        let expected = Environment::build(None, NONE.to_string(), terrain.terrain());

        assert_eq!(
            Environment::from(&terrain, BiomeArg::Default, &PathBuf::new())?,
            expected
        );

        Ok(())
    }

    #[test]
    fn environment_from_example_terrain_selected_biome() -> Result<()> {
        let terrain_dir = tempdir()?;
        create_dir_all(terrain_dir.path().join("tests/scripts"))?;

        let mut expected_envs = expected_env_vars_example_biome(terrain_dir.path());
        expected_envs.insert(
            "PROCESS_ENV_REF_VAR".to_string(),
            "PROCESS_ENV_VALUE".to_string(),
        );
        expected_envs.insert("SCRIPTS_DIR".to_string(), "scripts".to_string());
        expected_envs.insert("TEST_DIR".to_string(), "tests".to_string());

        // terrain bg constructors
        let mut expected_bg_constructors = vec![
            Command::new(
                "/bin/bash".to_string(),
                vec!["-c".to_string(), "./print_num_for_10_sec".to_string()],
                None,
                Some(fs::canonicalize(terrain_dir.path().join("tests/scripts"))?),
            ),
            Command::new(
                "/bin/bash".to_string(),
                vec!["-c".to_string(), "./print_num_for_10_sec".to_string()],
                None,
                Some(PathBuf::from("/tmp")),
            ),
        ];

        // biome bg constructors
        expected_bg_constructors.extend(expected_constructor_background_example_biome(
            terrain_dir.path(),
        ));

        // terrain bg destructors
        let mut expected_bg_destructors = vec![Command::new(
            "/bin/bash".to_string(),
            vec!["-c".to_string(), "./print_num_for_10_sec".to_string()],
            None,
            Some(fs::canonicalize(terrain_dir.path().join("tests/scripts"))?),
        )];
        // biome bg destructors
        expected_bg_destructors.extend(expected_destructor_background_example_biome(
            terrain_dir.path(),
        ));

        let expected_constructors = Commands::new(
            expected_constructor_foreground_example_biome(terrain_dir.path()),
            expected_bg_constructors,
        );
        let expected_destructors = Commands::new(
            expected_destructor_foreground_example_biome(terrain_dir.path()),
            expected_bg_destructors,
        );

        let expected: Environment = Environment::build(
            Some(EXAMPLE_BIOME.to_string()),
            EXAMPLE_BIOME.to_string(),
            &Biome::new(
                EXAMPLE_BIOME.to_string(),
                expected_envs,
                expected_aliases_example_biome(),
                expected_constructors,
                expected_destructors,
            ),
        );

        let mut terrain = Terrain::example();
        terrain.terrain_mut().add_envs(vec![
            ("PROCESS_ENV_REF_VAR", "${PROCESS_ENV_VAR}"),
            ("SCRIPTS_DIR", "scripts"),
            ("TEST_DIR", "tests"),
        ]);

        terrain.terrain_mut().add_bg_constructors(vec![
            Command::new(
                "/bin/bash".to_string(),
                vec!["-c".to_string(), "./print_num_for_10_sec".to_string()],
                None,
                Some(PathBuf::from("tests/scripts")),
            ),
            Command::new(
                "/bin/bash".to_string(),
                vec!["-c".to_string(), "./print_num_for_10_sec".to_string()],
                None,
                Some(PathBuf::from("/tmp")),
            ),
        ]);
        terrain.terrain_mut().add_bg_destructors(vec![Command::new(
            "/bin/bash".to_string(),
            vec!["-c".to_string(), "./print_num_for_10_sec".to_string()],
            None,
            Some(PathBuf::from("${TEST_DIR}/${SCRIPTS_DIR}")),
        )]);

        let old_env = set_env_var(
            "PROCESS_ENV_VAR".to_string(),
            Some("PROCESS_ENV_VALUE".to_string()),
        );

        let actual = Environment::from(
            &terrain,
            BiomeArg::Some(EXAMPLE_BIOME.to_string()),
            terrain_dir.path(),
        )
        .expect("no error to be thrown");

        assert_eq!(actual, expected);

        restore_env_var("PROCESS_ENV_VAR".to_string(), old_env);
        Ok(())
    }

    #[test]
    fn environment_from_example_terrain_default() -> Result<()> {
        let terrain_dir = tempdir()?;

        let expected: Environment = Environment::build(
            Some(EXAMPLE_BIOME.to_string()),
            EXAMPLE_BIOME.to_string(),
            &Biome::new(
                EXAMPLE_BIOME.to_string(),
                expected_env_vars_example_biome(terrain_dir.path()),
                expected_aliases_example_biome(),
                expected_constructors_example_biome(terrain_dir.path()),
                expected_destructors_example_biome(terrain_dir.path()),
            ),
        );

        let actual = Environment::from(&Terrain::example(), BiomeArg::Default, terrain_dir.path())
            .expect("no error to be thrown");

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn environment_from_example_terrain_none_selected() -> Result<()> {
        let terrain_dir = tempdir()?;

        let mut terrain = Terrain::example();
        terrain.terrain_mut().insert_env(
            "TERRAIN_DIR".to_string(),
            terrain_dir.path().to_string_lossy().to_string(),
        );
        terrain
            .terrain_mut()
            .insert_env("TERRAIN_SELECTED_BIOME".to_string(), "none".to_string());
        terrain.terrain_mut().substitute_envs();
        terrain.terrain_mut().substitute_cwd(terrain_dir.path())?;

        let expected: Environment = Environment::build(
            Some("example_biome".to_string()),
            "none".to_string(),
            terrain.terrain(),
        );

        let actual = Environment::from(&Terrain::example(), BiomeArg::None, terrain_dir.path())
            .expect("no error to be thrown");

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn environment_from_terrain_throws_error_if_selected_biome_does_not_exists() -> Result<()> {
        let error = Environment::from(
            &Terrain::default(),
            BiomeArg::Some("non_existent_biome".to_string()),
            &PathBuf::new(),
        )
        .expect_err("expected an error when selected_biome does not exists")
        .to_string();

        assert_eq!("the biome \"non_existent_biome\" does not exists", error);

        Ok(())
    }

    #[test]
    fn environment_from_example_terrain_selected_biome_different_from_default() -> Result<()> {
        let terrain_dir = tempdir()?;

        let mut expected_envs: BTreeMap<String, String> = BTreeMap::new();
        expected_envs.insert(
            "TERRAIN_DIR".to_string(),
            terrain_dir.path().to_string_lossy().to_string(),
        );
        expected_envs.insert(
            "TERRAIN_SELECTED_BIOME".to_string(),
            "example_biome2".to_string(),
        );
        expected_envs.insert("EDITOR".to_string(), "nano".to_string());
        expected_envs.insert("ENV_VAR".to_string(), "env_val".to_string());
        expected_envs.insert(
            "NESTED_POINTER".to_string(),
            "env_val-env_val-${NULL}".to_string(),
        );
        expected_envs.insert("NULL_POINTER".to_string(), "${NULL}".to_string());
        expected_envs.insert("PAGER".to_string(), "less".to_string());
        expected_envs.insert("POINTER_ENV_VAR".to_string(), "env_val".to_string());

        let mut expected_aliases: BTreeMap<String, String> = BTreeMap::new();
        expected_aliases.insert(
            "tenter".to_string(),
            "terrainium enter --biome example_biome2".to_string(),
        );
        expected_aliases.insert("texit".to_string(), "terrainium exit".to_string());
        let expected_constructor_foreground: Vec<Command> = vec![
            Command::new(
                "/bin/echo".to_string(),
                vec!["entering terrain".to_string()],
                None,
                Some(terrain_dir.path().to_path_buf()),
            ),
            Command::new(
                "/bin/echo".to_string(),
                vec!["entering biome example_biome2".to_string()],
                None,
                Some(terrain_dir.path().to_path_buf()),
            ),
        ];
        let expected_constructor_background: Vec<Command> = vec![];
        let expected_destructor_foreground: Vec<Command> = vec![
            Command::new(
                "/bin/echo".to_string(),
                vec!["exiting terrain".to_string()],
                None,
                Some(terrain_dir.path().to_path_buf()),
            ),
            Command::new(
                "/bin/echo".to_string(),
                vec!["exiting biome example_biome2".to_string()],
                None,
                Some(terrain_dir.path().to_path_buf()),
            ),
        ];
        let expected_destructor_background: Vec<Command> = vec![];

        let expected_constructor = Commands::new(
            expected_constructor_foreground,
            expected_constructor_background,
        );
        let expected_destructor = Commands::new(
            expected_destructor_foreground,
            expected_destructor_background,
        );

        let expected: Environment = Environment::build(
            Some(EXAMPLE_BIOME.to_string()),
            "example_biome2".to_string(),
            &Biome::new(
                "example_biome2".to_string(),
                expected_envs,
                expected_aliases,
                expected_constructor,
                expected_destructor,
            ),
        );

        let mut terrain = Terrain::example();
        add_biome(
            &mut terrain,
            "example_biome2".to_string(),
            get_test_biome("example_biome2".to_string(), "nano".to_string()),
        );

        let actual = Environment::from(
            &terrain,
            BiomeArg::Some("example_biome2".to_string()),
            terrain_dir.path(),
        )
        .expect("no error to be thrown");

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn environment_to_get_template() {
        let environment =
            Environment::from(&Terrain::example(), BiomeArg::Default, &PathBuf::new())
                .expect("not to fail");

        let base_template = fs::read_to_string("./templates/get.hbs").expect("to be read");
        let envs_template = fs::read_to_string("./templates/get_env.hbs").expect("to be read");
        let aliases_template =
            fs::read_to_string("./templates/get_aliases.hbs").expect("to be read");
        let constructors_template =
            fs::read_to_string("./templates/get_constructors.hbs").expect("to be read");
        let destructors_template =
            fs::read_to_string("./templates/get_destructors.hbs").expect("to be read");

        let mut templates: BTreeMap<String, String> = BTreeMap::new();
        templates.insert("get".to_string(), base_template);
        templates.insert("envs".to_string(), envs_template);
        templates.insert("aliases".to_string(), aliases_template);
        templates.insert("constructors".to_string(), constructors_template);
        templates.insert("destructors".to_string(), destructors_template);

        let rendered = environment
            .to_rendered("get".to_string(), templates)
            .expect("no error to be thrown");

        assert_eq!(
            fs::read_to_string("./tests/data/terrain-example_biome.rendered")
                .expect("test data file to be read"),
            rendered
        )
    }

    #[test]
    fn environment_to_zsh() {
        let environment = Environment::from(
            &Terrain::example(),
            BiomeArg::Default,
            &PathBuf::from("/home/user/work/terrainium"),
        )
        .expect("not to fail");

        let rendered = environment
            .to_rendered("zsh".to_string(), Zsh::templates())
            .expect("no error to be thrown");

        assert_eq!(
            fs::read_to_string("./tests/data/terrain-example_biome.example.zsh")
                .expect("test data file to be read"),
            rendered
        )
    }

    #[test]
    fn validate_envs() {
        let mut environment =
            Environment::from(&Terrain::default(), BiomeArg::Default, &PathBuf::new())
                .expect("not to fail");

        let mut envs: BTreeMap<String, String> = BTreeMap::new();
        envs.insert("EDITOR".to_string(), "nano".to_string());
        envs.insert(
            "NESTED_POINTER".to_string(),
            "env_val-${NULL_1}-${NULL_2}".to_string(),
        );

        environment.merged_mut().set_envs(envs);

        let messages = environment.validate().expect("should not fail").results();

        assert_eq!(messages.len(), 1);
        assert!(messages.contains(&ValidationResult {
            level: ValidationMessageLevel::Warn,
            message: "environment variable 'NESTED_POINTER' contains reference to variables \
                 ('NULL_1', 'NULL_2') that are not defined in terrain.toml and system environment variables. \
                 ensure that variables ('NULL_1', 'NULL_2') are set before using 'NESTED_POINTER' environment variable.".to_string(),
            r#for: "none".to_string(),
            fix_action: ValidationFixAction::None,
        }));
    }
}
