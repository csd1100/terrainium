use crate::client::types::biome::Biome;
use crate::client::types::terrain::Terrain;
use anyhow::{Context, Result};
use handlebars::Handlebars;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize, Debug, PartialEq)]
pub struct Environment {
    default_biome: Option<String>,
    selected_biome: String,
    merged: Biome,
}

impl Environment {
    pub fn from(terrain: &Terrain, selected_biome: Option<String>) -> Result<Self> {
        let merged: Biome = terrain.merged(&selected_biome)?;

        let selected = selected_biome.unwrap_or_else(|| {
            if terrain.default_biome().is_none() {
                "none".to_string()
            } else {
                "default".to_string()
            }
        });

        Ok(Environment {
            default_biome: terrain.default_biome().clone(),
            selected_biome: selected,
            merged,
        })
    }

    pub(crate) fn to_rendered(
        &self,
        main_template: String,
        templates: BTreeMap<String, String>,
    ) -> Result<String> {
        render(main_template, templates, self)
    }

    #[cfg(test)]
    pub fn build(default_biome: Option<String>, selected_biome: String, merged: &Biome) -> Self {
        Environment {
            default_biome,
            selected_biome,
            merged: merged.clone(),
        }
    }
}

pub fn render<T: Serialize>(
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
mod test {
    use crate::client::types::biome::Biome;
    use crate::client::types::commands::Commands;
    use crate::client::types::environment::Environment;
    use crate::client::types::terrain::test::{
        add_biome, force_set_invalid_default_biome, get_test_biome,
    };
    use crate::client::types::terrain::Terrain;
    use crate::common::types::command::Command;
    use anyhow::Result;
    use std::collections::BTreeMap;
    use std::fs;

    #[test]
    fn environment_from_empty_terrain() -> Result<()> {
        let expected: Environment =
            Environment::build(None, "none".to_string(), Terrain::default().terrain());

        let actual = Environment::from(&Terrain::default(), None).expect("no error to be thrown");

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn environment_from_example_but_no_default_or_selected() -> Result<()> {
        let mut terrain = Terrain::example();
        force_set_invalid_default_biome(&mut terrain, None);
        let expected = Environment::build(None, "none".to_string(), terrain.terrain());

        assert_eq!(Environment::from(&terrain, None)?, expected);

        Ok(())
    }

    #[test]
    fn environment_from_example_terrain_selected_biome() -> Result<()> {
        let mut expected_envs: BTreeMap<String, String> = BTreeMap::new();
        expected_envs.insert("EDITOR".to_string(), "nvim".to_string());
        expected_envs.insert("PAGER".to_string(), "less".to_string());
        let mut expected_aliases: BTreeMap<String, String> = BTreeMap::new();
        expected_aliases.insert(
            "tenter".to_string(),
            "terrainium enter --biome example_biome".to_string(),
        );
        expected_aliases.insert("texit".to_string(), "terrainium exit".to_string());
        let expected_constructor_foreground: Vec<Command> = vec![
            Command::new(
                "/bin/echo".to_string(),
                vec!["entering terrain".to_string()],
            ),
            Command::new(
                "/bin/echo".to_string(),
                vec!["entering biome example_biome".to_string()],
            ),
        ];
        let expected_constructor_background: Vec<Command> = vec![];
        let expected_destructor_foreground: Vec<Command> = vec![
            Command::new("/bin/echo".to_string(), vec!["exiting terrain".to_string()]),
            Command::new(
                "/bin/echo".to_string(),
                vec!["exiting biome example_biome".to_string()],
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
            Some("example_biome".to_string()),
            "example_biome".to_string(),
            &Biome::new(
                expected_envs,
                expected_aliases,
                expected_constructor,
                expected_destructor,
            ),
        );

        let actual = Environment::from(&Terrain::example(), Some("example_biome".to_string()))
            .expect("no error to be thrown");

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn environment_from_example_terrain_default() -> Result<()> {
        let mut expected_envs: BTreeMap<String, String> = BTreeMap::new();
        expected_envs.insert("EDITOR".to_string(), "nvim".to_string());
        expected_envs.insert("PAGER".to_string(), "less".to_string());
        let mut expected_aliases: BTreeMap<String, String> = BTreeMap::new();
        expected_aliases.insert(
            "tenter".to_string(),
            "terrainium enter --biome example_biome".to_string(),
        );
        expected_aliases.insert("texit".to_string(), "terrainium exit".to_string());
        let expected_constructor_foreground: Vec<Command> = vec![
            Command::new(
                "/bin/echo".to_string(),
                vec!["entering terrain".to_string()],
            ),
            Command::new(
                "/bin/echo".to_string(),
                vec!["entering biome example_biome".to_string()],
            ),
        ];
        let expected_constructor_background: Vec<Command> = vec![];
        let expected_destructor_foreground: Vec<Command> = vec![
            Command::new("/bin/echo".to_string(), vec!["exiting terrain".to_string()]),
            Command::new(
                "/bin/echo".to_string(),
                vec!["exiting biome example_biome".to_string()],
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
            Some("example_biome".to_string()),
            "default".to_string(),
            &Biome::new(
                expected_envs,
                expected_aliases,
                expected_constructor,
                expected_destructor,
            ),
        );

        let actual = Environment::from(&Terrain::example(), None).expect("no error to be thrown");

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn environment_from_example_terrain_none_selected() -> Result<()> {
        let expected: Environment = Environment::build(
            Some("example_biome".to_string()),
            "none".to_string(),
            Terrain::example().terrain(),
        );

        let actual = Environment::from(&Terrain::example(), Some("none".to_string()))
            .expect("no error to be thrown");

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn environment_from_terrain_throws_error_if_selected_biome_does_not_exists() -> Result<()> {
        let error = Environment::from(&Terrain::default(), Some("non_existent_biome".to_string()))
            .expect_err("expected an error when selected_biome does not exists")
            .to_string();

        assert_eq!("the biome \"non_existent_biome\" does not exists", error);

        Ok(())
    }

    #[test]
    fn environment_from_example_terrain_selected_biome_different_from_default() -> Result<()> {
        let mut expected_envs: BTreeMap<String, String> = BTreeMap::new();
        expected_envs.insert("EDITOR".to_string(), "nano".to_string());
        expected_envs.insert("PAGER".to_string(), "less".to_string());
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
            ),
            Command::new(
                "/bin/echo".to_string(),
                vec!["entering biome example_biome2".to_string()],
            ),
        ];
        let expected_constructor_background: Vec<Command> = vec![];
        let expected_destructor_foreground: Vec<Command> = vec![
            Command::new("/bin/echo".to_string(), vec!["exiting terrain".to_string()]),
            Command::new(
                "/bin/echo".to_string(),
                vec!["exiting biome example_biome2".to_string()],
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
            Some("example_biome".to_string()),
            "example_biome2".to_string(),
            &Biome::new(
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

        let actual = Environment::from(&terrain, Some("example_biome2".to_string()))
            .expect("no error to be thrown");

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn environment_to_get_template() {
        let environment = Environment::from(&Terrain::example(), Some("example_biome".to_string()))
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
        let base_template =
            fs::read_to_string("./templates/zsh_final_script.hbs").expect("to be read");
        let envs_template = fs::read_to_string("./templates/zsh_env.hbs").expect("to be read");
        let aliases_template =
            fs::read_to_string("./templates/zsh_aliases.hbs").expect("to be read");
        let constructors_template =
            fs::read_to_string("./templates/zsh_constructors.hbs").expect("to be read");
        let destructors_template =
            fs::read_to_string("./templates/zsh_destructors.hbs").expect("to be read");

        let mut templates: BTreeMap<String, String> = BTreeMap::new();
        templates.insert("zsh".to_string(), base_template);
        templates.insert("envs".to_string(), envs_template);
        templates.insert("aliases".to_string(), aliases_template);
        templates.insert("constructors".to_string(), constructors_template);
        templates.insert("destructors".to_string(), destructors_template);

        let environment = Environment::from(&Terrain::example(), Some("example_biome".to_string()))
            .expect("not to fail");

        let rendered = environment
            .to_rendered("zsh".to_string(), templates)
            .expect("no error to be thrown");

        assert_eq!(
            fs::read_to_string("./tests/data/terrain-example_biome.example.zsh")
                .expect("test data file to be read"),
            rendered
        )
    }
}
