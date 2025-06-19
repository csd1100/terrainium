use crate::client::args::GetArgs;
use crate::client::types::context::Context;
use crate::client::types::environment::Environment;
use crate::client::types::terrain::{AutoApply, Terrain};
use anyhow::{Context as AnyhowContext, Result};

pub fn handle(context: Context, terrain: Terrain, get_args: GetArgs) -> Result<()> {
    let output = get(context, terrain, get_args)?;
    print!("{output}");
    Ok(())
}

fn get(context: Context, terrain: Terrain, get_args: GetArgs) -> Result<String> {
    let environment = Environment::from(&terrain, get_args.biome.clone(), context.terrain_dir())
        .context("failed to generate environment")?;

    if get_args.empty() {
        if get_args.json {
            return serde_json::to_string_pretty(&environment)
                .context("failed to convert environment to json");
        }
        return Ok(format!("{environment}"));
    }

    if get_args.auto_apply {
        if context.config().auto_apply() {
            return Ok(terrain.auto_apply().to_string());
        }
        return Ok(AutoApply::default().to_string());
    }

    let mut result = String::new();

    if get_args.envs {
        result += &environment.merged().envs_str(None);
    } else if !get_args.env.is_empty() {
        result += &environment.merged().envs_str(Some(&get_args.env));
    }

    if get_args.aliases {
        result += &environment.merged().aliases_str(None);
    } else if !get_args.alias.is_empty() {
        result += &environment.merged().aliases_str(Some(&get_args.alias));
    }

    if get_args.constructors {
        result += &environment.merged().constructors_str();
    }

    if get_args.destructors {
        result += &environment.merged().destructors_str();
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use crate::client::args::{BiomeArg, GetArgs};
    use crate::client::types::config::Config;
    use crate::client::types::context::Context;
    use crate::client::types::terrain::tests::set_auto_apply;
    use crate::client::types::terrain::Terrain;
    use crate::common::constants::EXAMPLE_BIOME;
    use crate::common::execute::MockExecutor;
    use anyhow::Result;
    use std::fs::read_to_string;
    use std::path::Path;
    use std::str::FromStr;

    #[test]
    fn get_all_for_default_biome() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Default,
            aliases: false,
            envs: false,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, Terrain::example(), args).expect("to not throw an error");

        let expected =
            read_to_string("./tests/data/terrain-default.rendered").expect("test data to be read");

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_all_for_empty_terrain() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Default,
            aliases: false,
            envs: false,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, Terrain::default(), args).expect("to not throw an error");

        let expected =
            read_to_string("./tests/data/terrain-empty.rendered").expect("test data to be read");

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_all_for_selected_biome() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::from_str(EXAMPLE_BIOME)?,
            aliases: false,
            envs: false,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, Terrain::example(), args).expect("to not throw an error");

        let expected = read_to_string("./tests/data/terrain-example_biome.rendered")
            .expect("test data to be read");

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_all_for_default_biome_json() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: true,
            biome: BiomeArg::Default,
            aliases: false,
            envs: false,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, Terrain::example(), args).expect("to not throw an error");

        let expected = read_to_string("./tests/data/terrain-example_biome.json")
            .expect("test data to be read");

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_all_aliases_for_default_biome() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Default,
            aliases: true,
            envs: false,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, Terrain::example(), args).expect("to not throw an error");
        let expected = r#"Aliases:
    tenter="terrainium enter --biome example_biome"
    texit="terrainium exit"
"#;

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_all_aliases_for_selected_biome() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Some(EXAMPLE_BIOME.to_string()),
            aliases: true,
            envs: false,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, Terrain::example(), args).expect("to not throw an error");
        let expected = r#"Aliases:
    tenter="terrainium enter --biome example_biome"
    texit="terrainium exit"
"#;

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_all_aliases_for_empty() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::None,
            aliases: true,
            envs: false,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, Terrain::default(), args).expect("to not throw an error");
        let expected = "";

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_all_envs_for_default_biome() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Default,
            aliases: false,
            envs: true,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, Terrain::example(), args).expect("to not throw an error");
        let expected = r#"Environment Variables:
    EDITOR="nvim"
    ENV_VAR="overridden_env_val"
    NESTED_POINTER="overridden_env_val-overridden_env_val-${NULL}"
    NULL_POINTER="${NULL}"
    PAGER="less"
    POINTER_ENV_VAR="overridden_env_val"
    TERRAIN_DIR=""
    TERRAIN_SELECTED_BIOME="example_biome"
"#;

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_all_envs_for_selected_biome() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Some(EXAMPLE_BIOME.to_string()),
            aliases: false,
            envs: true,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, Terrain::example(), args).expect("to not throw an error");
        let expected = r#"Environment Variables:
    EDITOR="nvim"
    ENV_VAR="overridden_env_val"
    NESTED_POINTER="overridden_env_val-overridden_env_val-${NULL}"
    NULL_POINTER="${NULL}"
    PAGER="less"
    POINTER_ENV_VAR="overridden_env_val"
    TERRAIN_DIR=""
    TERRAIN_SELECTED_BIOME="example_biome"
"#;

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_all_envs_for_empty() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Default,
            aliases: false,
            envs: true,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, Terrain::default(), args).expect("to not throw an error");
        let expected = r#"Environment Variables:
    TERRAIN_DIR=""
    TERRAIN_SELECTED_BIOME="none"
"#;

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_all_envs_and_aliases_for_default_biome() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Default,
            aliases: true,
            envs: true,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, Terrain::example(), args).expect("to not throw an error");
        let expected = r#"Environment Variables:
    EDITOR="nvim"
    ENV_VAR="overridden_env_val"
    NESTED_POINTER="overridden_env_val-overridden_env_val-${NULL}"
    NULL_POINTER="${NULL}"
    PAGER="less"
    POINTER_ENV_VAR="overridden_env_val"
    TERRAIN_DIR=""
    TERRAIN_SELECTED_BIOME="example_biome"
Aliases:
    tenter="terrainium enter --biome example_biome"
    texit="terrainium exit"
"#;

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_env() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Default,
            aliases: false,
            envs: false,
            alias: vec![],
            env: vec!["EDITOR".to_string(), "NON_EXISTENT".to_string()],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, Terrain::example(), args).expect("to not throw an error");
        let expected = r#"Environment Variables:
    EDITOR="nvim"
    NON_EXISTENT="!!!DOES NOT EXIST!!!"
"#;

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_alias() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Default,
            aliases: false,
            envs: false,
            alias: vec!["tenter".to_string(), "non_existent".to_string()],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, Terrain::example(), args).expect("to not throw an error");
        let expected = r#"Aliases:
    non_existent="!!!DOES NOT EXIST!!!"
    tenter="terrainium enter --biome example_biome"
"#;

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_constructors() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Default,
            aliases: false,
            envs: false,
            alias: vec![],
            env: vec![],
            constructors: true,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, Terrain::example(), args).expect("to not throw an error");

        let expected = r#"Constructors:
    foreground:
        `/bin/echo entering terrain` in terrain directory
        `/bin/echo entering biome example_biome` in terrain directory
    background:
        `/bin/bash -c ${PWD}/tests/scripts/print_num_for_10_sec` in terrain directory
"#;

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_destructors() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Default,
            aliases: false,
            envs: false,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: true,
            auto_apply: false,
        };

        let output = super::get(context, Terrain::example(), args).expect("to not throw an error");

        let expected = r#"Destructors:
    foreground:
        `/bin/echo exiting terrain` in terrain directory
        `/bin/echo exiting biome example_biome` in terrain directory
    background:
        `/bin/bash -c ${TERRAIN_DIR}/tests/scripts/print_num_for_10_sec` in terrain directory
"#;

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_all_envs_aliases_constructors_destructors() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Default,
            aliases: true,
            envs: true,
            alias: vec![],
            env: vec![],
            constructors: true,
            destructors: true,
            auto_apply: false,
        };

        let output = super::get(context, Terrain::example(), args).expect("to not throw an error");

        let expected = r#"Environment Variables:
    EDITOR="nvim"
    ENV_VAR="overridden_env_val"
    NESTED_POINTER="overridden_env_val-overridden_env_val-${NULL}"
    NULL_POINTER="${NULL}"
    PAGER="less"
    POINTER_ENV_VAR="overridden_env_val"
    TERRAIN_DIR=""
    TERRAIN_SELECTED_BIOME="example_biome"
Aliases:
    tenter="terrainium enter --biome example_biome"
    texit="terrainium exit"
Constructors:
    foreground:
        `/bin/echo entering terrain` in terrain directory
        `/bin/echo entering biome example_biome` in terrain directory
    background:
        `/bin/bash -c ${PWD}/tests/scripts/print_num_for_10_sec` in terrain directory
Destructors:
    foreground:
        `/bin/echo exiting terrain` in terrain directory
        `/bin/echo exiting biome example_biome` in terrain directory
    background:
        `/bin/bash -c ${TERRAIN_DIR}/tests/scripts/print_num_for_10_sec` in terrain directory
"#;

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_env_alias_constructors_destructors() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Default,
            aliases: false,
            envs: false,
            alias: vec!["tenter".to_string(), "non_existent".to_string()],
            env: vec!["EDITOR".to_string(), "NON_EXISTENT".to_string()],
            constructors: true,
            destructors: true,
            auto_apply: false,
        };

        let output = super::get(context, Terrain::example(), args).expect("to not throw an error");

        let expected = r#"Environment Variables:
    EDITOR="nvim"
    NON_EXISTENT="!!!DOES NOT EXIST!!!"
Aliases:
    non_existent="!!!DOES NOT EXIST!!!"
    tenter="terrainium enter --biome example_biome"
Constructors:
    foreground:
        `/bin/echo entering terrain` in terrain directory
        `/bin/echo entering biome example_biome` in terrain directory
    background:
        `/bin/bash -c ${PWD}/tests/scripts/print_num_for_10_sec` in terrain directory
Destructors:
    foreground:
        `/bin/echo exiting terrain` in terrain directory
        `/bin/echo exiting biome example_biome` in terrain directory
    background:
        `/bin/bash -c ${TERRAIN_DIR}/tests/scripts/print_num_for_10_sec` in terrain directory
"#;

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_env_and_all_alias() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Default,
            aliases: true,
            envs: false,
            alias: vec![],
            env: vec!["EDITOR".to_string(), "NON_EXISTENT".to_string()],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, Terrain::example(), args).expect("to not throw an error");
        let expected = r#"Environment Variables:
    EDITOR="nvim"
    NON_EXISTENT="!!!DOES NOT EXIST!!!"
Aliases:
    tenter="terrainium enter --biome example_biome"
    texit="terrainium exit"
"#;

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_alias_and_all_envs() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Default,
            aliases: false,
            envs: true,
            alias: vec!["tenter".to_string(), "non_existent".to_string()],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: false,
        };

        let output = super::get(context, Terrain::example(), args).expect("to not throw an error");
        let expected = r#"Environment Variables:
    EDITOR="nvim"
    ENV_VAR="overridden_env_val"
    NESTED_POINTER="overridden_env_val-overridden_env_val-${NULL}"
    NULL_POINTER="${NULL}"
    PAGER="less"
    POINTER_ENV_VAR="overridden_env_val"
    TERRAIN_DIR=""
    TERRAIN_SELECTED_BIOME="example_biome"
Aliases:
    non_existent="!!!DOES NOT EXIST!!!"
    tenter="terrainium enter --biome example_biome"
"#;

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_auto_apply() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Default,
            aliases: true,
            envs: false,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: true,
        };

        let mut terrain = Terrain::example();
        set_auto_apply(&mut terrain, "enabled");

        let output = super::get(context, terrain, args).expect("to not throw an error");
        let expected = "enabled";

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_auto_apply_replace() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Default,
            aliases: false,
            envs: true,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: true,
        };

        let mut terrain = Terrain::example();
        set_auto_apply(&mut terrain, "replace");

        let output = super::get(context, terrain, args).expect("to not throw an error");
        let expected = "replace";

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_auto_apply_background() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Default,
            aliases: false,
            envs: true,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: true,
        };

        let mut terrain = Terrain::example();
        set_auto_apply(&mut terrain, "background");

        let output = super::get(context, terrain, args).expect("to not throw an error");
        let expected = "background";

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_auto_apply_off() -> Result<()> {
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::new());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Default,
            aliases: false,
            envs: true,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: true,
        };

        let mut terrain = Terrain::example();
        set_auto_apply(&mut terrain, "off");

        let output = super::get(context, terrain, args).expect("to not throw an error");
        let expected = "off";

        assert_eq!(output, expected);

        Ok(())
    }

    #[test]
    fn get_auto_apply_globally_off() -> Result<()> {
        let context = Context::build_with_config(Config::auto_apply_off());

        let args = GetArgs {
            json: false,
            biome: BiomeArg::Default,
            aliases: true,
            envs: false,
            alias: vec![],
            env: vec![],
            constructors: false,
            destructors: false,
            auto_apply: true,
        };

        let mut terrain = Terrain::example();
        set_auto_apply(&mut terrain, "all");

        let output = super::get(context, terrain, args).expect("to not throw an error");
        let expected = "off";

        assert_eq!(output, expected);

        Ok(())
    }
}
