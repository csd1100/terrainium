use std::collections::HashMap;

use anyhow::{Context, Result};
use mockall_double::double;

use crate::types::args::{BiomeArg, GetOpts};

#[double]
use crate::helpers::operations::fs;

#[double]
use crate::templates::get::print;

pub fn handle(biome: Option<BiomeArg>, opts: GetOpts) -> Result<()> {
    let terrain = fs::get_parsed_terrain()?;
    if opts.is_empty() {
        let mut terrain = terrain
            .get_printable_terrain(biome)
            .context("failed to get printable terrain")?;
        terrain.all = true;
        print::all(terrain)?;
    } else {
        let GetOpts {
            alias_all,
            alias,
            env_all,
            env,
            constructors,
            destructors,
        } = opts;
        let terrain = terrain.get(biome)?;
        if alias_all {
            print::aliases(terrain.alias.to_owned()).context("unable to print aliases")?;
        } else if let Some(alias) = alias {
            let found_alias = Some(
                terrain
                    .find_aliases(alias)
                    .context("unable to get aliases")?,
            );
            let aliases: HashMap<String, String> = found_alias
                .expect("to be present")
                .iter()
                .map(|(k, v)| {
                    if v.is_none() {
                        (k.to_string(), "NOT FOUND".to_string())
                    } else {
                        (
                            k.to_string(),
                            v.to_owned().expect("to be present").to_string(),
                        )
                    }
                })
                .collect();
            print::aliases(Some(aliases)).context("unable to print aliases")?;
        }

        if env_all {
            print::env(terrain.env.to_owned()).context("unable to print env vars")?;
        } else if let Some(env) = env {
            let found_env = Some(terrain.find_envs(env).context("unable to get env vars")?);
            let env: HashMap<String, String> = found_env
                .expect("to be present")
                .iter()
                .map(|(k, v)| {
                    if v.is_none() {
                        (k.to_string(), "NOT FOUND".to_string())
                    } else {
                        (
                            k.to_string(),
                            v.to_owned().expect("to be present").to_string(),
                        )
                    }
                })
                .collect();
            print::env(Some(env)).context("unable to print env vars")?;
        }

        if constructors {
            print::constructors(terrain.constructors).context("unable to print constructors")?;
        }
        if destructors {
            print::destructors(terrain.destructors).context("unable to print destructors")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use anyhow::Result;
    use mockall::predicate::eq;
    use serial_test::serial;

    use crate::{
        helpers::operations::mock_fs,
        templates::get::mock_print,
        types::{
            args::{BiomeArg, GetOpts},
            commands::{Command, Commands},
            terrain::test_data,
        },
    };

    #[test]
    #[serial]
    fn get_all_calls_print_all() -> Result<()> {
        let mock_get_parsed_terrain = mock_fs::get_parsed_terrain_context();
        mock_get_parsed_terrain
            .expect()
            .return_once(|| Ok(test_data::terrain_full()))
            .times(1);

        let mock_print_all = mock_print::all_context();

        let mut printable = test_data::terrain_full().get_printable_terrain(None)?;
        printable.all = true;

        mock_print_all
            .expect()
            .with(eq(printable))
            .return_once(|_| Ok(()));

        let opts = GetOpts {
            alias_all: false,
            alias: None,
            env_all: false,
            env: None,
            constructors: false,
            destructors: false,
        };
        super::handle(None, opts)?;
        Ok(())
    }

    #[test]
    #[serial]
    fn get_env_all_calls_print_env_with_all() -> Result<()> {
        let mock_get_parsed_terrain = mock_fs::get_parsed_terrain_context();
        mock_get_parsed_terrain
            .expect()
            .return_once(|| Ok(test_data::terrain_full()))
            .times(1);

        let mock_print_env = mock_print::env_context();

        let mut envs = HashMap::<String, String>::new();
        envs.insert("EDITOR".to_string(), "nvim".to_string());
        envs.insert("TEST".to_string(), "value".to_string());

        mock_print_env
            .expect()
            .with(eq(Some(envs)))
            .return_once(|_| Ok(()));

        let opts = GetOpts {
            alias_all: false,
            alias: None,
            env_all: true,
            env: None,
            constructors: false,
            destructors: false,
        };

        super::handle(None, opts)?;
        Ok(())
    }

    #[test]
    #[serial]
    fn get_env_calls_print_env() -> Result<()> {
        let mock_get_parsed_terrain = mock_fs::get_parsed_terrain_context();
        mock_get_parsed_terrain
            .expect()
            .return_once(|| Ok(test_data::terrain_full()))
            .times(1);

        let mock_print_env = mock_print::env_context();

        let mut envs = HashMap::<String, String>::new();
        envs.insert("EDITOR".to_string(), "nvim".to_string());
        envs.insert("NONEXISTENT".to_string(), "NOT FOUND".to_string());

        mock_print_env
            .expect()
            .with(eq(Some(envs)))
            .return_once(|_| Ok(()));

        let opts = GetOpts {
            alias_all: false,
            alias: None,
            env_all: false,
            env: Some(vec!["EDITOR".to_string(), "NONEXISTENT".to_string()]),
            constructors: false,
            destructors: false,
        };

        super::handle(None, opts)?;
        Ok(())
    }

    #[test]
    #[serial]
    fn get_alias_all_calls_print_alias_with_all() -> Result<()> {
        let mock_get_parsed_terrain = mock_fs::get_parsed_terrain_context();
        mock_get_parsed_terrain
            .expect()
            .return_once(|| Ok(test_data::terrain_full()))
            .times(1);

        let mock_print_aliases = mock_print::aliases_context();

        let mut aliases = HashMap::<String, String>::new();
        aliases.insert("tedit".to_string(), "terrainium edit".to_string());
        aliases.insert(
            "tenter".to_string(),
            "terrainium enter --biome example_biome2".to_string(),
        );

        mock_print_aliases
            .expect()
            .with(eq(Some(aliases)))
            .return_once(|_| Ok(()));

        let opts = GetOpts {
            alias_all: true,
            alias: None,
            env_all: false,
            env: None,
            constructors: false,
            destructors: false,
        };

        super::handle(Some(BiomeArg::Value("example_biome2".to_string())), opts)?;
        Ok(())
    }

    #[test]
    #[serial]
    fn get_alias_calls_print_alias() -> Result<()> {
        let mock_get_parsed_terrain = mock_fs::get_parsed_terrain_context();
        mock_get_parsed_terrain
            .expect()
            .return_once(|| Ok(test_data::terrain_full()))
            .times(1);

        let mock_print_aliases = mock_print::aliases_context();

        let mut aliases = HashMap::<String, String>::new();
        aliases.insert("NONEXISTENT".to_string(), "NOT FOUND".to_string());
        aliases.insert(
            "tenter".to_string(),
            "terrainium enter --biome example_biome2".to_string(),
        );

        mock_print_aliases
            .expect()
            .with(eq(Some(aliases)))
            .return_once(|_| Ok(()));

        let opts = GetOpts {
            alias_all: false,
            alias: Some(vec!["tenter".to_string(), "NONEXISTENT".to_string()]),
            env_all: false,
            env: None,
            constructors: false,
            destructors: false,
        };

        super::handle(Some(BiomeArg::Value("example_biome2".to_string())), opts)?;
        Ok(())
    }

    #[test]
    #[serial]
    fn get_constructors_calls_print_constructors() -> Result<()> {
        let mock_get_parsed_terrain = mock_fs::get_parsed_terrain_context();
        mock_get_parsed_terrain
            .expect()
            .return_once(|| Ok(test_data::terrain_full()))
            .times(1);

        let mock_print_constructors = mock_print::constructors_context();

        let constructors = Commands {
            background: Some(vec![Command {
                exe: "run".to_string(),
                args: Some(vec!["something".to_string()]),
            }]),
            foreground: Some(vec![Command {
                exe: "echo".to_string(),
                args: Some(vec!["entering terrain".to_string()]),
            }]),
        };

        mock_print_constructors
            .expect()
            .with(eq(Some(constructors)))
            .return_once(|_| Ok(()));

        let opts = GetOpts {
            alias_all: false,
            alias: None,
            env_all: false,
            env: None,
            constructors: true,
            destructors: false,
        };

        super::handle(Some(BiomeArg::None), opts)?;
        Ok(())
    }

    #[test]
    #[serial]
    fn get_destructors_calls_print_destructors() -> Result<()> {
        let mock_get_parsed_terrain = mock_fs::get_parsed_terrain_context();
        mock_get_parsed_terrain
            .expect()
            .return_once(|| Ok(test_data::terrain_full()))
            .times(1);

        let mock_print_destructors = mock_print::destructors_context();

        let destructors = Commands {
            background: Some(vec![Command {
                exe: "stop".to_string(),
                args: Some(vec!["something".to_string()]),
            }]),
            foreground: Some(vec![Command {
                exe: "echo".to_string(),
                args: Some(vec!["exiting terrain".to_string()]),
            }]),
        };

        mock_print_destructors
            .expect()
            .with(eq(Some(destructors)))
            .return_once(|_| Ok(()));

        let opts = GetOpts {
            alias_all: false,
            alias: None,
            env_all: false,
            env: None,
            constructors: false,
            destructors: true,
        };

        super::handle(Some(BiomeArg::None), opts)?;
        Ok(())
    }

    #[test]
    #[serial]
    fn get_mix() -> Result<()> {
        let mock_get_parsed_terrain = mock_fs::get_parsed_terrain_context();
        mock_get_parsed_terrain
            .expect()
            .return_once(|| Ok(test_data::terrain_full()))
            .times(1);

        let mock_print_env = mock_print::env_context();
        let mut envs = HashMap::<String, String>::new();
        envs.insert("EDITOR".to_string(), "vim".to_string());
        envs.insert("NONEXISTENT".to_string(), "NOT FOUND".to_string());
        mock_print_env
            .expect()
            .with(eq(Some(envs)))
            .return_once(|_| Ok(()));

        let mock_print_aliases = mock_print::aliases_context();
        let mut aliases = HashMap::<String, String>::new();
        aliases.insert("NONEXISTENT".to_string(), "NOT FOUND".to_string());
        aliases.insert("tenter".to_string(), "terrainium enter".to_string());
        mock_print_aliases
            .expect()
            .with(eq(Some(aliases)))
            .return_once(|_| Ok(()));

        let mock_print_constructors = mock_print::constructors_context();
        let constructors = Commands {
            background: Some(vec![Command {
                exe: "run".to_string(),
                args: Some(vec!["something".to_string()]),
            }]),
            foreground: Some(vec![Command {
                exe: "echo".to_string(),
                args: Some(vec!["entering terrain".to_string()]),
            }]),
        };
        mock_print_constructors
            .expect()
            .with(eq(Some(constructors)))
            .return_once(|_| Ok(()));

        let mock_print_destructors = mock_print::destructors_context();
        let destructors = Commands {
            background: Some(vec![Command {
                exe: "stop".to_string(),
                args: Some(vec!["something".to_string()]),
            }]),
            foreground: Some(vec![Command {
                exe: "echo".to_string(),
                args: Some(vec!["exiting terrain".to_string()]),
            }]),
        };
        mock_print_destructors
            .expect()
            .with(eq(Some(destructors)))
            .return_once(|_| Ok(()));

        let opts = GetOpts {
            alias_all: false,
            alias: Some(vec!["tenter".to_string(), "NONEXISTENT".to_string()]),
            env_all: false,
            env: Some(vec!["EDITOR".to_string(), "NONEXISTENT".to_string()]),
            constructors: true,
            destructors: true,
        };

        super::handle(Some(BiomeArg::None), opts)?;
        Ok(())
    }
}
