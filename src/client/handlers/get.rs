use crate::client::args::{option_string_from, GetArgs};
use crate::client::types::context::Context;
use crate::client::types::environment::{render, Environment};
use crate::client::types::terrain::Terrain;
use crate::common::constants::{
    DOES_NOT_EXIST, GET_ALIASES_TEMPLATE_NAME, GET_CONSTRUCTORS_TEMPLATE_NAME,
    GET_DESTRUCTORS_TEMPLATE_NAME, GET_ENVS_TEMPLATE_NAME, GET_MAIN_TEMPLATE_NAME,
};
use anyhow::{Context as AnyhowContext, Result};
use std::collections::BTreeMap;
use std::fs::read_to_string;

const GET_MAIN_TEMPLATE: &str = include_str!("../../../templates/get.hbs");
const GET_ENVS_TEMPLATE: &str = include_str!("../../../templates/get_env.hbs");
const GET_ALIASES_TEMPLATE: &str = include_str!("../../../templates/get_aliases.hbs");
const GET_CONSTRUCTORS_TEMPLATE: &str = include_str!("../../../templates/get_constructors.hbs");
const GET_DESTRUCTORS_TEMPLATE: &str = include_str!("../../../templates/get_destructors.hbs");

pub fn handle(context: Context, get_args: GetArgs) -> Result<()> {
    let output = get(context, get_args)?;
    print!("{}", output);
    Ok(())
}

fn get(context: Context, get_args: GetArgs) -> Result<String> {
    let toml_path = context.toml_path()?;
    let selected_biome = option_string_from(&get_args.biome);

    let terrain =
        Terrain::from_toml(read_to_string(&toml_path).context("failed to read terrain.toml")?)
            .expect("terrain to be parsed from toml");

    let mut result = String::new();

    if get_args.empty() {
        result += &all(&terrain, &selected_biome)?;
        return Ok(result);
    }

    if get_args.auto_apply {
        result += if terrain.auto_apply().is_enabled() {
            "true"
        } else if terrain.auto_apply().is_replace() {
            "replace"
        } else {
            "false"
        };
        return Ok(result);
    }

    if get_args.aliases {
        result += &all_aliases(&terrain, &selected_biome)?;
    } else if !get_args.alias.is_empty() {
        result += &alias(&get_args, &terrain, &selected_biome)?;
    }

    if get_args.envs {
        result += &all_envs(&terrain, &selected_biome)?;
    } else if !get_args.env.is_empty() {
        result += &env(&get_args, &terrain, &selected_biome)?;
    }

    if get_args.constructors {
        result += &constructors(&terrain, &selected_biome)?;
    }

    if get_args.destructors {
        result += &destructors(terrain, &selected_biome)?;
    }

    Ok(result)
}

fn destructors(terrain: Terrain, selected_biome: &Option<String>) -> Result<String> {
    let constructors = terrain.merged_destructors(selected_biome)?;
    Ok(render(
        GET_DESTRUCTORS_TEMPLATE_NAME.to_string(),
        templates(),
        constructors,
    )
    .expect("failed to render envs in get template"))
}

fn constructors(terrain: &Terrain, selected_biome: &Option<String>) -> Result<String> {
    let constructors = terrain.merged_constructors(selected_biome)?;
    Ok(render(
        GET_CONSTRUCTORS_TEMPLATE_NAME.to_string(),
        templates(),
        constructors,
    )
    .expect("failed to render envs in get template"))
}

fn env(get_args: &GetArgs, terrain: &Terrain, selected_biome: &Option<String>) -> Result<String> {
    let envs = terrain.merged_envs(selected_biome)?;
    let mut requested = BTreeMap::<String, String>::new();

    get_args.env.clone().iter().for_each(|env| {
        if let Some(value) = envs.get(env) {
            requested.insert(env.to_string(), value.to_string());
        } else {
            requested.insert(env.to_string(), DOES_NOT_EXIST.to_string());
        }
    });

    Ok(
        render(GET_ENVS_TEMPLATE_NAME.to_string(), templates(), requested)
            .expect("failed to render envs in get template"),
    )
}

fn alias(get_args: &GetArgs, terrain: &Terrain, selected_biome: &Option<String>) -> Result<String> {
    let aliases = terrain.merged_aliases(selected_biome)?;
    let mut requested = BTreeMap::<String, String>::new();

    get_args.alias.clone().iter().for_each(|alias| {
        if let Some(value) = aliases.get(alias) {
            requested.insert(alias.to_string(), value.to_string());
        } else {
            requested.insert(alias.to_string(), DOES_NOT_EXIST.to_string());
        }
    });

    Ok(render(
        GET_ALIASES_TEMPLATE_NAME.to_string(),
        templates(),
        requested,
    )
    .expect("failed to render aliases in get template"))
}

fn all_envs(terrain: &Terrain, selected_biome: &Option<String>) -> Result<String> {
    let envs = terrain.merged_envs(selected_biome)?;
    Ok(
        render(GET_ENVS_TEMPLATE_NAME.to_string(), templates(), envs)
            .expect("failed to render envs in get template"),
    )
}

fn all_aliases(terrain: &Terrain, selected_biome: &Option<String>) -> Result<String> {
    let aliases = terrain.merged_aliases(selected_biome)?;
    Ok(
        render(GET_ALIASES_TEMPLATE_NAME.to_string(), templates(), aliases)
            .expect("failed to render aliases in get template"),
    )
}

fn all(terrain: &Terrain, selected_biome: &Option<String>) -> Result<String> {
    let environment = Environment::from(terrain, selected_biome.clone())
        .context("failed to generate environment from terrain")?;

    let res = environment
        .to_rendered(GET_MAIN_TEMPLATE_NAME.to_string(), templates())
        .expect("get output to be rendered");
    Ok(res)
}

fn templates() -> BTreeMap<String, String> {
    let mut templates: BTreeMap<String, String> = BTreeMap::new();
    templates.insert(
        GET_MAIN_TEMPLATE_NAME.to_string(),
        GET_MAIN_TEMPLATE.to_string(),
    );
    templates.insert(
        GET_ENVS_TEMPLATE_NAME.to_string(),
        GET_ENVS_TEMPLATE.to_string(),
    );
    templates.insert(
        GET_ALIASES_TEMPLATE_NAME.to_string(),
        GET_ALIASES_TEMPLATE.to_string(),
    );
    templates.insert(
        GET_CONSTRUCTORS_TEMPLATE_NAME.to_string(),
        GET_CONSTRUCTORS_TEMPLATE.to_string(),
    );
    templates.insert(
        GET_DESTRUCTORS_TEMPLATE_NAME.to_string(),
        GET_DESTRUCTORS_TEMPLATE.to_string(),
    );
    templates
}

#[cfg(test)]
mod test {
    use crate::client::args::{BiomeArg, GetArgs};
    use crate::client::shell::Zsh;
    use crate::client::types::context::Context;
    use crate::common::execute::MockCommandToRun;
    use anyhow::Result;
    use serial_test::serial;
    use std::fs::{copy, read_to_string};
    use std::path::PathBuf;
    use std::str::FromStr;
    use tempfile::tempdir;

    #[serial]
    #[test]
    fn get_all_for_default_biome() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied");

        let args = GetArgs {
            biome: None,
            aliases: false,
            envs: false,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, args).expect("to not throw an error");

        let expected =
            read_to_string("./tests/data/terrain-default.rendered").expect("test data to be read");

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_all_for_empty_terrain() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        copy("./tests/data/terrain.empty.toml", &terrain_toml).expect("test terrain to be copied");

        let args = GetArgs {
            biome: None,
            aliases: false,
            envs: false,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, args).expect("to not throw an error");

        let expected =
            read_to_string("./tests/data/terrain-empty.rendered").expect("test data to be read");

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_all_for_selected_biome() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied");

        let args = GetArgs {
            biome: Some(BiomeArg::from_str("example_biome")?),
            aliases: false,
            envs: false,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, args).expect("to not throw an error");

        let expected = read_to_string("./tests/data/terrain-example_biome.rendered")
            .expect("test data to be read");

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_all_aliases_for_default_biome() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied");

        let args = GetArgs {
            biome: None,
            aliases: true,
            envs: false,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, args).expect("to not throw an error");
        let expected = "Aliases:\n    tenter=\"terrainium enter --biome example_biome\"\n    texit=\"terrainium exit\"\n";

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_all_aliases_for_selected_biome() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied");

        let args = GetArgs {
            biome: Some(BiomeArg::Some("example_biome".to_string())),
            aliases: true,
            envs: false,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, args).expect("to not throw an error");
        let expected = "Aliases:\n    tenter=\"terrainium enter --biome example_biome\"\n    texit=\"terrainium exit\"\n";

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_all_aliases_for_empty() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        copy("./tests/data/terrain.empty.toml", &terrain_toml).expect("test terrain to be copied");

        let args = GetArgs {
            biome: None,
            aliases: true,
            envs: false,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, args).expect("to not throw an error");
        let expected = "";

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_all_envs_for_default_biome() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied");

        let args = GetArgs {
            biome: None,
            aliases: false,
            envs: true,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, args).expect("to not throw an error");
        let expected = "Environment Variables:\n    EDITOR=\"nvim\"\n    PAGER=\"less\"\n";

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_all_envs_for_selected_biome() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied");

        let args = GetArgs {
            biome: Some(BiomeArg::Some("example_biome".to_string())),
            aliases: false,
            envs: true,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, args).expect("to not throw an error");
        let expected = "Environment Variables:\n    EDITOR=\"nvim\"\n    PAGER=\"less\"\n";

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_all_envs_for_empty() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        copy("./tests/data/terrain.empty.toml", &terrain_toml).expect("test terrain to be copied");

        let args = GetArgs {
            biome: None,
            aliases: false,
            envs: true,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, args).expect("to not throw an error");
        let expected = "";

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_all_envs_and_aliases_for_default_biome() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied");

        let args = GetArgs {
            biome: None,
            aliases: true,
            envs: true,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, args).expect("to not throw an error");
        let expected = "Aliases:\n    tenter=\"terrainium enter --biome example_biome\"\n    texit=\"terrainium exit\"\nEnvironment Variables:\n    EDITOR=\"nvim\"\n    PAGER=\"less\"\n";

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_env() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied");

        let args = GetArgs {
            biome: None,
            aliases: false,
            envs: false,
            alias: vec![],
            env: vec!["EDITOR".to_string(), "NON_EXISTENT".to_string()],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, args).expect("to not throw an error");
        let expected = "Environment Variables:\n    EDITOR=\"nvim\"\n    NON_EXISTENT=\"!!!DOES NOT EXIST!!!\"\n";

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_alias() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied");

        let args = GetArgs {
            biome: None,
            aliases: false,
            envs: false,
            alias: vec!["tenter".to_string(), "non_existent".to_string()],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, args).expect("to not throw an error");
        let expected = "Aliases:\n    non_existent=\"!!!DOES NOT EXIST!!!\"\n    tenter=\"terrainium enter --biome example_biome\"\n";

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_constructors() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied");

        let args = GetArgs {
            biome: None,
            aliases: false,
            envs: false,
            alias: vec![],
            env: vec![],
            constructors: true,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, args).expect("to not throw an error");

        let expected = "Constructors:\n    background:\n        /bin/bash -c $PWD/tests/scripts/print_num_for_10_sec \n    foreground:\n        /bin/echo entering terrain \n        /bin/echo entering biome example_biome \n";

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_destructors() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied");

        let args = GetArgs {
            biome: None,
            aliases: false,
            envs: false,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: true,
            auto_apply: false,
        };

        let output = super::get(context, args).expect("to not throw an error");

        let expected = "Destructors:\n    background:\n        /bin/bash -c $PWD/tests/scripts/print_num_for_10_sec \n    foreground:\n        /bin/echo exiting terrain \n        /bin/echo exiting biome example_biome \n";

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_all_envs_aliases_constructors_destructors() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied");

        let args = GetArgs {
            biome: None,
            aliases: true,
            envs: true,
            alias: vec![],
            env: vec![],
            constructors: true,
            destructors: true,
            auto_apply: false,
        };

        let output = super::get(context, args).expect("to not throw an error");

        let mut expected = "Aliases:\n    tenter=\"terrainium enter --biome example_biome\"\n    texit=\"terrainium exit\"\nEnvironment Variables:\n    EDITOR=\"nvim\"\n    PAGER=\"less\"\n".to_string();
        expected += "Constructors:\n    background:\n        /bin/bash -c $PWD/tests/scripts/print_num_for_10_sec \n    foreground:\n        /bin/echo entering terrain \n        /bin/echo entering biome example_biome \n";
        expected += "Destructors:\n    background:\n        /bin/bash -c $PWD/tests/scripts/print_num_for_10_sec \n    foreground:\n        /bin/echo exiting terrain \n        /bin/echo exiting biome example_biome \n";

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_env_alias_constructors_destructors() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied");

        let args = GetArgs {
            biome: None,
            aliases: false,
            envs: false,
            alias: vec!["tenter".to_string(), "non_existent".to_string()],
            env: vec!["EDITOR".to_string(), "NON_EXISTENT".to_string()],
            constructors: true,
            destructors: true,
            auto_apply: false,
        };

        let output = super::get(context, args).expect("to not throw an error");

        let mut expected = "Aliases:\n    non_existent=\"!!!DOES NOT EXIST!!!\"\n    tenter=\"terrainium enter --biome example_biome\"\n".to_string();
        expected += "Environment Variables:\n    EDITOR=\"nvim\"\n    NON_EXISTENT=\"!!!DOES NOT EXIST!!!\"\n";
        expected += "Constructors:\n    background:\n        /bin/bash -c $PWD/tests/scripts/print_num_for_10_sec \n    foreground:\n        /bin/echo entering terrain \n        /bin/echo entering biome example_biome \n";
        expected += "Destructors:\n    background:\n        /bin/bash -c $PWD/tests/scripts/print_num_for_10_sec \n    foreground:\n        /bin/echo exiting terrain \n        /bin/echo exiting biome example_biome \n";

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_env_and_all_alias() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied");

        let args = GetArgs {
            biome: None,
            aliases: true,
            envs: false,
            alias: vec![],
            env: vec!["EDITOR".to_string(), "NON_EXISTENT".to_string()],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, args).expect("to not throw an error");
        let mut expected = "Aliases:\n    tenter=\"terrainium enter --biome example_biome\"\n    texit=\"terrainium exit\"\n".to_string();
        expected += "Environment Variables:\n    EDITOR=\"nvim\"\n    NON_EXISTENT=\"!!!DOES NOT EXIST!!!\"\n";

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_alias_and_all_envs() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied");

        let args = GetArgs {
            biome: None,
            aliases: false,
            envs: true,
            alias: vec!["tenter".to_string(), "non_existent".to_string()],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, args).expect("to not throw an error");
        let mut expected = "Aliases:\n    non_existent=\"!!!DOES NOT EXIST!!!\"\n    tenter=\"terrainium enter --biome example_biome\"\n".to_string();
        expected += "Environment Variables:\n    EDITOR=\"nvim\"\n    PAGER=\"less\"\n";

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_auto_apply() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        copy(
            "./tests/data/terrain.example.auto_apply.enabled.toml",
            &terrain_toml,
        )
        .expect("test terrain to be copied");

        let args = GetArgs {
            biome: None,
            aliases: true,
            envs: false,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: true,
        };

        let output = super::get(context, args).expect("to not throw an error");
        let expected = "true";

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_auto_apply_replace() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        copy(
            "./tests/data/terrain.example.auto_apply.replace.toml",
            &terrain_toml,
        )
        .expect("test terrain to be copied");

        let args = GetArgs {
            biome: None,
            aliases: false,
            envs: true,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: true,
        };

        let output = super::get(context, args).expect("to not throw an error");
        let expected = "replace";

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }
}
