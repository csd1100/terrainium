use crate::client::args::{option_string_from, Pair, UpdateArgs};
use crate::client::shell::Shell;
use crate::client::types::biome::Biome;
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use anyhow::{anyhow, Context as AnyhowContext, Result};
use std::collections::BTreeMap;
use std::fs::{copy, read_to_string, write};

pub fn handle(context: Context, update_args: UpdateArgs) -> Result<()> {
    let mut terrain = Terrain::from_toml(
        read_to_string(context.toml_path()?).context("failed to read terrain.toml")?,
    )
    .expect("failed to parse terrain from toml");

    if update_args.auto_apply.is_some() {
        terrain.set_auto_apply(update_args.auto_apply.expect("auto_apply to be present"));
    }

    if update_args.set_default.is_some() {
        let new_default = update_args
            .set_default
            .expect("new default biome value to be some");
        if !terrain.biomes().contains_key(&new_default) {
            return Err(anyhow!(
                "cannot update default biome to '{}', biome '{}' does not exists",
                &new_default,
                &new_default
            ));
        }
        terrain.set_default(new_default);
    } else {
        let mut biome: Biome = Biome::default();

        if !update_args.env.is_empty() {
            biome.set_envs(map_from_pair(&update_args.env))
        }

        if !update_args.alias.is_empty() {
            biome.set_aliases(map_from_pair(&update_args.alias))
        }

        if update_args.new.is_some() {
            let biome_name = update_args.new.expect("new biome to be some");
            terrain.update(biome_name, biome);
        } else {
            let (name, selected) = terrain.select_biome(&option_string_from(&update_args.biome))?;
            let updated = selected.merge(&biome);
            terrain.update(name, updated);
        }
    }

    if update_args.backup {
        let mut backup = context.toml_path()?;
        backup.set_extension("toml.bkp");

        copy(context.toml_path()?, backup).context("failed to backup terrain.toml")?;
    }

    write(
        context.toml_path()?,
        terrain.to_toml().expect("to generate toml from terrain"),
    )
    .context("failed to write updated terrain to file")?;

    context.shell().generate_scripts(&context, terrain)?;

    Ok(())
}

fn map_from_pair(pairs: &[Pair]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::<String, String>::new();
    pairs.iter().for_each(|pair| {
        let _ = map.insert(pair.key.to_string(), pair.value.to_string());
    });
    map
}

#[cfg(test)]
mod test {
    use crate::client::args::{BiomeArg, Pair, UpdateArgs};
    use crate::client::old_utils::test::script_path;
    use crate::client::shell::Zsh;
    use crate::client::types::context::Context;
    use crate::client::types::terrain::AutoApply;
    use crate::client::utils::ExpectShell;
    use crate::common::execute::MockCommandToRun;
    use std::fs::{copy, create_dir_all, exists, read_to_string};
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn set_default_biome() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy(
            "./tests/data/terrain.example.without.default.toml",
            &terrain_toml,
        )
        .expect("test terrain to be copied to test dir");

        let expected_shell_operation = ExpectShell::to()
            .compile_script_for("example_biome", central_dir.path())
            .compile_script_for("none", central_dir.path())
            .successfully();

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        create_dir_all(context.scripts_dir()).expect("test scripts dir to be created");

        super::handle(
            context,
            UpdateArgs {
                set_default: Some("example_biome".to_string()),
                biome: None,
                alias: vec![],
                env: vec![],
                new: None,
                backup: false,
                auto_apply: None,
            },
        )
        .expect("no error to be thrown");

        let actual = read_to_string(&terrain_toml).expect("updated terrain to be read");
        let expected =
            read_to_string("./tests/data/terrain.example.toml").expect("test terrain to be read");

        assert_eq!(actual, expected);

        // assert example_biome script is created
        let script: PathBuf = script_path(central_dir.path(), &"example_biome".to_string());

        assert!(
            exists(&script).expect("to check if script exists"),
            "expected terrain-example_biome.zsh to be created in scripts directory"
        );

        let actual = read_to_string(&script).expect("expected script to be readable");
        let expected = read_to_string("./tests/data/terrain-example_biome.example.zsh")
            .expect("expected test toml to be readable");

        assert_eq!(actual, expected);

        // assert none script is created
        let script_path: PathBuf = script_path(central_dir.path(), &"none".to_string());
        assert!(
            exists(&script_path).expect("to check if script exists"),
            "expected terrain-none.zsh to be created in current directory"
        );

        let actual_script =
            read_to_string(&script_path).expect("expected terrain-none.zsh to be readable");
        let expected_script = read_to_string("./tests/data/terrain-none.example.zsh")
            .expect("expected test script to be readable");

        assert_eq!(actual_script, expected_script);
    }

    #[test]
    fn set_default_biome_invalid() {
        let current_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy(
            "./tests/data/terrain.example.without.default.toml",
            &terrain_toml,
        )
        .expect("test terrain to be copied to test dir");

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
        );

        let err = super::handle(
            context,
            UpdateArgs {
                set_default: Some("non_existent".to_string()),
                biome: None,
                alias: vec![],
                env: vec![],
                new: None,
                backup: false,
                auto_apply: None,
            },
        )
        .expect_err("error to be thrown")
        .to_string();

        assert_eq!(
            "cannot update default biome to 'non_existent', biome 'non_existent' does not exists",
            err
        );

        // assert terrain not updated in case of error
        let actual = read_to_string(&terrain_toml).expect("updated terrain to be read");
        let expected = read_to_string("./tests/data/terrain.example.without.default.toml")
            .expect("test terrain to be read");

        assert_eq!(actual, expected);
    }

    #[test]
    fn new_biome() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let expected_shell_operation = ExpectShell::to()
            .compile_script_for("example_biome", central_dir.path())
            .compile_script_for("example_biome2", central_dir.path())
            .compile_script_for("none", central_dir.path())
            .successfully();

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        create_dir_all(context.scripts_dir()).expect("test scripts dir to be created");

        super::handle(
            context,
            UpdateArgs {
                set_default: None,
                biome: None,
                alias: vec![
                    Pair {
                        key: "tenter".to_string(),
                        value: "terrainium enter --biome example_biome2".to_string(),
                    },
                    Pair {
                        key: "new".to_string(),
                        value: "new_alias".to_string(),
                    },
                ],
                env: vec![
                    Pair {
                        key: "EDITOR".to_string(),
                        value: "nano".to_string(),
                    },
                    Pair {
                        key: "NEW".to_string(),
                        value: "VALUE".to_string(),
                    },
                ],
                new: Some("example_biome2".to_string()),
                backup: false,
                auto_apply: None,
            },
        )
        .expect("no error to be thrown");

        let actual = read_to_string(&terrain_toml).expect("updated terrain to be read");
        let expected = read_to_string("./tests/data/terrain.example.new.example_biome2.toml")
            .expect("test terrain to be read");

        assert_eq!(actual, expected);

        // assert example_biome script is created
        let script: PathBuf = script_path(central_dir.path(), &"example_biome".to_string());

        assert!(
            exists(&script).expect("to check if script exists"),
            "expected terrain-example_biome.zsh to be created in scripts directory"
        );

        let actual = read_to_string(&script).expect("expected script to be readable");
        let expected = read_to_string("./tests/data/terrain-example_biome.example.zsh")
            .expect("expected test toml to be readable");

        assert_eq!(actual, expected);

        // assert example_biome2 script is created
        let script: PathBuf = script_path(central_dir.path(), &"example_biome2".to_string());

        assert!(
            exists(&script).expect("to check if script exists"),
            "expected terrain-example_biome.zsh to be created in scripts directory"
        );

        let actual = read_to_string(&script).expect("expected script to be readable");
        let expected = read_to_string("./tests/data/terrain-example_biome2.example.zsh")
            .expect("expected test toml to be readable");

        assert_eq!(actual, expected);

        // assert none script is created
        let script_path: PathBuf = script_path(central_dir.path(), &"none".to_string());
        assert!(
            exists(&script_path).expect("to check if script exists"),
            "expected terrain-none.zsh to be created in current directory"
        );

        let actual_script =
            read_to_string(&script_path).expect("expected terrain-none.zsh to be readable");
        let expected_script = read_to_string("./tests/data/terrain-none.example.zsh")
            .expect("expected test script to be readable");

        assert_eq!(actual_script, expected_script);
    }

    #[test]
    fn update_biome() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let expected_shell_operation = ExpectShell::to()
            .compile_script_for("example_biome", central_dir.path())
            .compile_script_for("none", central_dir.path())
            .successfully();

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        create_dir_all(context.scripts_dir()).expect("test scripts dir to be created");

        super::handle(
            context,
            UpdateArgs {
                set_default: None,
                biome: None,
                alias: vec![Pair {
                    key: "greet".to_string(),
                    value: "echo hello".to_string(),
                }],
                env: vec![Pair {
                    key: "EDITOR".to_string(),
                    value: "nano".to_string(),
                }],
                new: None,
                backup: false,
                auto_apply: None,
            },
        )
        .expect("no error to be thrown");

        let actual = read_to_string(&terrain_toml).expect("updated terrain to be read");
        let expected = read_to_string("./tests/data/terrain.example.updated.toml")
            .expect("test terrain to be read");

        assert_eq!(actual, expected);

        // assert example_biome script is created
        let script: PathBuf = script_path(central_dir.path(), &"example_biome".to_string());

        assert!(
            exists(&script).expect("to check if script exists"),
            "expected terrain-example_biome.zsh to be created in scripts directory"
        );

        let actual = read_to_string(&script).expect("expected script to be readable");
        let expected = read_to_string("./tests/data/terrain-example_biome.example.updated.zsh")
            .expect("expected test toml to be readable");

        assert_eq!(actual, expected);

        // assert none script is created
        let script_path: PathBuf = script_path(central_dir.path(), &"none".to_string());
        assert!(
            exists(&script_path).expect("to check if script exists"),
            "expected terrain-none.zsh to be created in current directory"
        );

        let actual_script =
            read_to_string(&script_path).expect("expected terrain-none.zsh to be readable");
        let expected_script = read_to_string("./tests/data/terrain-none.example.zsh")
            .expect("expected test script to be readable");

        assert_eq!(actual_script, expected_script);
    }

    #[test]
    fn update_main() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let expected_shell_operation = ExpectShell::to()
            .compile_script_for("example_biome", central_dir.path())
            .compile_script_for("none", central_dir.path())
            .successfully();

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        create_dir_all(context.scripts_dir()).expect("test scripts dir to be created");

        super::handle(
            context,
            UpdateArgs {
                set_default: None,
                biome: Some(BiomeArg::None),
                alias: vec![Pair {
                    key: "greet".to_string(),
                    value: "echo hello".to_string(),
                }],
                env: vec![Pair {
                    key: "EDITOR".to_string(),
                    value: "nano".to_string(),
                }],
                new: None,
                backup: false,
                auto_apply: None,
            },
        )
        .expect("no error to be thrown");

        let actual = read_to_string(&terrain_toml).expect("updated terrain to be read");
        let expected = read_to_string("./tests/data/terrain.example.updated.none.toml")
            .expect("test terrain to be read");

        assert_eq!(actual, expected);

        // assert example_biome script is created
        let script: PathBuf = script_path(central_dir.path(), &"example_biome".to_string());

        assert!(
            exists(&script).expect("to check if script exists"),
            "expected terrain-example_biome.zsh to be created in scripts directory"
        );

        let actual = read_to_string(&script).expect("expected script to be readable");
        let expected =
            read_to_string("./tests/data/terrain-example_biome.example.updated.none.zsh")
                .expect("expected test toml to be readable");

        assert_eq!(actual, expected);

        // assert none script is created
        let script_path: PathBuf = script_path(central_dir.path(), &"none".to_string());
        assert!(
            exists(&script_path).expect("to check if script exists"),
            "expected terrain-none.zsh to be created in current directory"
        );

        let actual_script =
            read_to_string(&script_path).expect("expected terrain-none.zsh to be readable");
        let expected_script = read_to_string("./tests/data/terrain-none.example.updated.zsh")
            .expect("expected test script to be readable");

        assert_eq!(actual_script, expected_script);
    }

    #[test]
    fn update_biome_invalid() {
        let current_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
        );

        let err = super::handle(
            context,
            UpdateArgs {
                set_default: None,
                biome: Some(BiomeArg::Some("non_existent".to_string())),
                alias: vec![Pair {
                    key: "greet".to_string(),
                    value: "echo hello".to_string(),
                }],
                env: vec![Pair {
                    key: "EDITOR".to_string(),
                    value: "nano".to_string(),
                }],
                new: None,
                backup: false,
                auto_apply: None,
            },
        )
        .expect_err("no error to be thrown")
        .to_string();

        assert_eq!("the biome \"non_existent\" does not exists", err);

        // assert terrain not updated in case of error
        let actual = read_to_string(&terrain_toml).expect("updated terrain to be read");
        let expected =
            read_to_string("./tests/data/terrain.example.toml").expect("test terrain to be read");

        assert_eq!(actual, expected);
    }

    #[test]
    fn backup() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let expected_shell_operation = ExpectShell::to()
            .compile_script_for("example_biome", central_dir.path())
            .compile_script_for("none", central_dir.path())
            .successfully();

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        create_dir_all(context.scripts_dir()).expect("test scripts dir to be created");

        super::handle(
            context,
            UpdateArgs {
                set_default: None,
                biome: None,
                alias: vec![Pair {
                    key: "greet".to_string(),
                    value: "echo hello".to_string(),
                }],
                env: vec![Pair {
                    key: "EDITOR".to_string(),
                    value: "nano".to_string(),
                }],
                new: None,
                backup: true,
                auto_apply: None,
            },
        )
        .expect("no error to be thrown");

        let actual = read_to_string(&terrain_toml).expect("updated terrain to be read");
        let expected = read_to_string("./tests/data/terrain.example.updated.toml")
            .expect("test terrain to be read");

        assert_eq!(actual, expected);

        let backup: PathBuf = current_dir.path().join("terrain.toml.bkp");

        let actual = read_to_string(&backup).expect("backup terrain to be read");
        let expected =
            read_to_string("./tests/data/terrain.example.toml").expect("test terrain to be read");

        assert_eq!(actual, expected);

        // assert example_biome script is created
        let script: PathBuf = script_path(central_dir.path(), &"example_biome".to_string());

        assert!(
            exists(&script).expect("to check if script exists"),
            "expected terrain-example_biome.zsh to be created in scripts directory"
        );

        let actual = read_to_string(&script).expect("expected script to be readable");
        let expected = read_to_string("./tests/data/terrain-example_biome.example.updated.zsh")
            .expect("expected test toml to be readable");

        assert_eq!(actual, expected);

        // assert none script is created
        let script_path: PathBuf = script_path(central_dir.path(), &"none".to_string());
        assert!(
            exists(&script_path).expect("to check if script exists"),
            "expected terrain-none.zsh to be created in current directory"
        );

        let actual_script =
            read_to_string(&script_path).expect("expected terrain-none.zsh to be readable");
        let expected_script = read_to_string("./tests/data/terrain-none.example.zsh")
            .expect("expected test script to be readable");

        assert_eq!(actual_script, expected_script);
    }

    #[test]
    fn auto_apply() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let expected_shell_operation = ExpectShell::to()
            .compile_script_for("example_biome", central_dir.path())
            .compile_script_for("none", central_dir.path())
            .successfully();

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        create_dir_all(context.scripts_dir()).expect("test scripts dir to be created");

        super::handle(
            context,
            UpdateArgs {
                set_default: None,
                biome: None,
                alias: vec![],
                env: vec![],
                new: None,
                backup: true,
                auto_apply: Some(AutoApply::enabled()),
            },
        )
        .expect("no error to be thrown");

        let actual = read_to_string(&terrain_toml).expect("updated terrain to be read");
        let expected = read_to_string("./tests/data/terrain.example.auto_apply.enabled.toml")
            .expect("test terrain to be read");

        assert_eq!(actual, expected);

        let backup: PathBuf = current_dir.path().join("terrain.toml.bkp");

        let actual = read_to_string(&backup).expect("backup terrain to be read");
        let expected =
            read_to_string("./tests/data/terrain.example.toml").expect("test terrain to be read");

        assert_eq!(actual, expected);

        // assert example_biome script is created
        let script: PathBuf = script_path(central_dir.path(), &"example_biome".to_string());

        assert!(
            exists(&script).expect("to check if script exists"),
            "expected terrain-example_biome.zsh to be created in scripts directory"
        );

        let actual = read_to_string(&script).expect("expected script to be readable");
        let expected = read_to_string("./tests/data/terrain-example_biome.example.zsh")
            .expect("expected test toml to be readable");

        assert_eq!(actual, expected);

        // assert none script is created
        let script_path: PathBuf = script_path(central_dir.path(), &"none".to_string());
        assert!(
            exists(&script_path).expect("to check if script exists"),
            "expected terrain-none.zsh to be created in current directory"
        );

        let actual_script =
            read_to_string(&script_path).expect("expected terrain-none.zsh to be readable");
        let expected_script = read_to_string("./tests/data/terrain-none.example.zsh")
            .expect("expected test script to be readable");

        assert_eq!(actual_script, expected_script);
    }

    #[test]
    fn auto_apply_turn_off() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy(
            "./tests/data/terrain.example.auto_apply.enabled.toml",
            &terrain_toml,
        )
        .expect("test terrain to be copied to test dir");

        let expected_shell_operation = ExpectShell::to()
            .compile_script_for("example_biome", central_dir.path())
            .compile_script_for("none", central_dir.path())
            .successfully();

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        create_dir_all(context.scripts_dir()).expect("test scripts dir to be created");

        super::handle(
            context,
            UpdateArgs {
                set_default: None,
                biome: None,
                alias: vec![],
                env: vec![],
                new: None,
                backup: true,
                auto_apply: Some(AutoApply::default()),
            },
        )
        .expect("no error to be thrown");

        let actual = read_to_string(&terrain_toml).expect("updated terrain to be read");
        let expected =
            read_to_string("./tests/data/terrain.example.toml").expect("test terrain to be read");

        assert_eq!(actual, expected);

        let backup: PathBuf = current_dir.path().join("terrain.toml.bkp");

        let actual = read_to_string(&backup).expect("backup terrain to be read");
        let expected = read_to_string("./tests/data/terrain.example.auto_apply.enabled.toml")
            .expect("test terrain to be read");

        assert_eq!(actual, expected);

        // assert example_biome script is created
        let script: PathBuf = script_path(central_dir.path(), &"example_biome".to_string());

        assert!(
            exists(&script).expect("to check if script exists"),
            "expected terrain-example_biome.zsh to be created in scripts directory"
        );

        let actual = read_to_string(&script).expect("expected script to be readable");
        let expected = read_to_string("./tests/data/terrain-example_biome.example.zsh")
            .expect("expected test toml to be readable");

        assert_eq!(actual, expected);

        // assert none script is created
        let script_path: PathBuf = script_path(central_dir.path(), &"none".to_string());
        assert!(
            exists(&script_path).expect("to check if script exists"),
            "expected terrain-none.zsh to be created in current directory"
        );

        let actual_script =
            read_to_string(&script_path).expect("expected terrain-none.zsh to be readable");
        let expected_script = read_to_string("./tests/data/terrain-none.example.zsh")
            .expect("expected test script to be readable");

        assert_eq!(actual_script, expected_script);
    }
}
