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
    let environment = Environment::from(&terrain, selected_biome, context.terrain_dir())
        .context("failed to generate environment")?;

    let mut result = String::new();

    if get_args.empty() {
        result += &all(&environment)?;
        return Ok(result);
    }

    if get_args.auto_apply {
        result = terrain.auto_apply().clone().into();
        return Ok(result);
    }

    if get_args.aliases {
        result += &all_aliases(&environment)?;
    } else if !get_args.alias.is_empty() {
        result += &alias(&get_args, &environment)?;
    }

    if get_args.envs {
        result += &all_envs(&environment)?;
    } else if !get_args.env.is_empty() {
        result += &env(&get_args, &environment)?;
    }

    if get_args.constructors {
        result += &constructors(&environment)?;
    }

    if get_args.destructors {
        result += &destructors(&environment)?;
    }

    Ok(result)
}

fn destructors(environment: &Environment) -> Result<String> {
    let destructors = environment.destructors();
    Ok(render(
        GET_DESTRUCTORS_TEMPLATE_NAME.to_string(),
        templates(),
        destructors,
    )
    .expect("failed to render envs in get template"))
}

fn constructors(environment: &Environment) -> Result<String> {
    let constructors = environment.constructors();
    Ok(render(
        GET_CONSTRUCTORS_TEMPLATE_NAME.to_string(),
        templates(),
        constructors,
    )
    .expect("failed to render envs in get template"))
}

fn env(get_args: &GetArgs, environment: &Environment) -> Result<String> {
    let envs = environment.envs();
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

fn alias(get_args: &GetArgs, environment: &Environment) -> Result<String> {
    let aliases = environment.aliases();
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

fn all_envs(environment: &Environment) -> Result<String> {
    let envs = environment.envs();
    Ok(
        render(GET_ENVS_TEMPLATE_NAME.to_string(), templates(), envs)
            .expect("failed to render envs in get template"),
    )
}

fn all_aliases(environment: &Environment) -> Result<String> {
    let aliases = environment.aliases();
    Ok(
        render(GET_ALIASES_TEMPLATE_NAME.to_string(), templates(), aliases)
            .expect("failed to render aliases in get template"),
    )
}

fn all(environment: &Environment) -> Result<String> {
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
mod tests {
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
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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
        let expected = r#"Aliases:
    tenter="terrainium enter --biome example_biome"
    texit="terrainium exit"
"#;

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
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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
        let expected = r#"Aliases:
    tenter="terrainium enter --biome example_biome"
    texit="terrainium exit"
"#;

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
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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
        let expected = r#"Environment Variables:
    EDITOR="nvim"
    ENV_VAR="overridden_env_val"
    NESTED_POINTER="overridden_env_val-overridden_env_val-${NULL}"
    NULL_POINTER="${NULL}"
    PAGER="less"
    POINTER_ENV_VAR="overridden_env_val"
"#;

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
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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
        let expected = r#"Environment Variables:
    EDITOR="nvim"
    ENV_VAR="overridden_env_val"
    NESTED_POINTER="overridden_env_val-overridden_env_val-${NULL}"
    NULL_POINTER="${NULL}"
    PAGER="less"
    POINTER_ENV_VAR="overridden_env_val"
"#;

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
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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
        let expected = r#"Aliases:
    tenter="terrainium enter --biome example_biome"
    texit="terrainium exit"
Environment Variables:
    EDITOR="nvim"
    ENV_VAR="overridden_env_val"
    NESTED_POINTER="overridden_env_val-overridden_env_val-${NULL}"
    NULL_POINTER="${NULL}"
    PAGER="less"
    POINTER_ENV_VAR="overridden_env_val"
"#;

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
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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
        let expected = r#"Environment Variables:
    EDITOR="nvim"
    NON_EXISTENT="!!!DOES NOT EXIST!!!"
"#;

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
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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
        let expected = r#"Aliases:
    non_existent="!!!DOES NOT EXIST!!!"
    tenter="terrainium enter --biome example_biome"
"#;

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
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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

        let expected = r#"Constructors:
    background:
        /bin/bash -c ./print_num_for_10_sec 
        /bin/bash -c ./print_num_for_10_sec 
    foreground:
        /bin/echo entering terrain 
        /bin/echo entering biome example_biome 
"#;

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
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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

        let expected = r#"Destructors:
    background:
        /bin/bash -c ./print_num_for_10_sec 
    foreground:
        /bin/echo exiting terrain 
        /bin/echo exiting biome example_biome 
"#;

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
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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

        let expected = r#"Aliases:
    tenter="terrainium enter --biome example_biome"
    texit="terrainium exit"
Environment Variables:
    EDITOR="nvim"
    ENV_VAR="overridden_env_val"
    NESTED_POINTER="overridden_env_val-overridden_env_val-${NULL}"
    NULL_POINTER="${NULL}"
    PAGER="less"
    POINTER_ENV_VAR="overridden_env_val"
Constructors:
    background:
        /bin/bash -c ./print_num_for_10_sec 
        /bin/bash -c ./print_num_for_10_sec 
    foreground:
        /bin/echo entering terrain 
        /bin/echo entering biome example_biome 
Destructors:
    background:
        /bin/bash -c ./print_num_for_10_sec 
    foreground:
        /bin/echo exiting terrain 
        /bin/echo exiting biome example_biome 
"#;

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
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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

        let expected = r#"Aliases:
    non_existent="!!!DOES NOT EXIST!!!"
    tenter="terrainium enter --biome example_biome"
Environment Variables:
    EDITOR="nvim"
    NON_EXISTENT="!!!DOES NOT EXIST!!!"
Constructors:
    background:
        /bin/bash -c ./print_num_for_10_sec 
        /bin/bash -c ./print_num_for_10_sec 
    foreground:
        /bin/echo entering terrain 
        /bin/echo entering biome example_biome 
Destructors:
    background:
        /bin/bash -c ./print_num_for_10_sec 
    foreground:
        /bin/echo exiting terrain 
        /bin/echo exiting biome example_biome 
"#;

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
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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
        let expected = r#"Aliases:
    tenter="terrainium enter --biome example_biome"
    texit="terrainium exit"
Environment Variables:
    EDITOR="nvim"
    NON_EXISTENT="!!!DOES NOT EXIST!!!"
"#;

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
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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
        let expected = r#"Aliases:
    non_existent="!!!DOES NOT EXIST!!!"
    tenter="terrainium enter --biome example_biome"
Environment Variables:
    EDITOR="nvim"
    ENV_VAR="overridden_env_val"
    NESTED_POINTER="overridden_env_val-overridden_env_val-${NULL}"
    NULL_POINTER="${NULL}"
    PAGER="less"
    POINTER_ENV_VAR="overridden_env_val"
"#;

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
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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
        let expected = "enabled";

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
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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
        let expected = "replaced";

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_auto_apply_background() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
        copy(
            "./tests/data/terrain.example.auto_apply.background.toml",
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
        let expected = "background";

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }

    #[test]
    fn get_auto_apply_off() -> Result<()> {
        let current_dir = tempdir()?;

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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
            auto_apply: true,
        };

        let output = super::get(context, args).expect("to not throw an error");
        let expected = "off";

        assert_eq!(output, expected);

        current_dir
            .close()
            .expect("test directories to be cleaned up");

        Ok(())
    }
}
