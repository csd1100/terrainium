use crate::client::args::{option_string_from, Pair, UpdateArgs};
use crate::client::shell::Shell;
use crate::client::types::biome::Biome;
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use anyhow::{anyhow, Context as AnyhowContext, Result};
use std::collections::BTreeMap;
use std::fs::copy;

pub fn handle(context: Context, terrain: Terrain, update_args: UpdateArgs) -> Result<()> {
    let mut terrain = terrain;
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
        let mut backup = context.toml_path().to_path_buf();
        backup.set_extension("toml.bkp");

        copy(context.toml_path(), backup).context("failed to backup terrain.toml")?;
    }

    let validated_and_fixed = Terrain::store_and_get_fixed_terrain(&context, terrain)?;
    context
        .shell()
        .generate_scripts(&context, validated_and_fixed)?;

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
mod tests {
    use crate::client::args::{BiomeArg, Pair, UpdateArgs};
    use crate::client::shell::Zsh;
    use crate::client::types::context::Context;
    use crate::client::types::terrain::tests::{force_set_invalid_default_biome, set_auto_apply};
    use crate::client::types::terrain::{AutoApply, Terrain};
    use crate::client::utils::{
        AssertTerrain, ExpectShell, IN_CURRENT_DIR, WITHOUT_DEFAULT_BIOME_TOML,
        WITH_AUTO_APPLY_ENABLED_EXAMPLE_TOML, WITH_EXAMPLE_BIOME_UPDATED_EXAMPLE_TOML,
        WITH_EXAMPLE_TERRAIN_TOML, WITH_NEW_EXAMPLE_BIOME2_EXAMPLE_TOML,
        WITH_NONE_UPDATED_EXAMPLE_TOML,
    };
    use crate::common::execute::MockCommandToRun;
    use std::fs::{copy, create_dir_all};
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    #[test]
    fn set_default_biome() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
        copy(WITHOUT_DEFAULT_BIOME_TOML, &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for("example_biome", central_dir.path())
            .compile_terrain_script_for("none", central_dir.path())
            .successfully();

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(expected_shell_operation),
        );

        create_dir_all(context.scripts_dir()).expect("test scripts dir to be created");

        let mut terrain = Terrain::example();
        force_set_invalid_default_biome(&mut terrain, None);

        super::handle(
            context,
            terrain,
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

        AssertTerrain::with_dirs_and_existing(
            current_dir.path(),
            central_dir.path(),
            WITHOUT_DEFAULT_BIOME_TOML,
        )
        .was_updated(IN_CURRENT_DIR, WITH_EXAMPLE_TERRAIN_TOML)
        .script_was_created_for("none")
        .script_was_created_for("example_biome");
    }

    #[test]
    fn set_default_biome_invalid() {
        let current_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
        copy(WITHOUT_DEFAULT_BIOME_TOML, &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(MockCommandToRun::default()),
        );

        let mut terrain = Terrain::example();
        force_set_invalid_default_biome(&mut terrain, None);
        let err = super::handle(
            context,
            terrain,
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
        AssertTerrain::with_dirs_and_existing(
            current_dir.path(),
            Path::new(""),
            WITHOUT_DEFAULT_BIOME_TOML,
        )
        .was_not_updated(IN_CURRENT_DIR);
    }

    #[test]
    fn new_biome() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy(WITH_EXAMPLE_TERRAIN_TOML, &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for("example_biome", central_dir.path())
            .compile_terrain_script_for("example_biome2", central_dir.path())
            .compile_terrain_script_for("none", central_dir.path())
            .successfully();

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(expected_shell_operation),
        );

        create_dir_all(context.scripts_dir()).expect("test scripts dir to be created");

        super::handle(
            context,
            Terrain::example(),
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

        AssertTerrain::with_dirs_and_existing(
            current_dir.path(),
            central_dir.path(),
            WITH_EXAMPLE_TERRAIN_TOML,
        )
        .was_updated(IN_CURRENT_DIR, WITH_NEW_EXAMPLE_BIOME2_EXAMPLE_TOML)
        .script_was_created_for("none")
        .script_was_created_for("example_biome")
        .script_was_created_for("example_biome2");
    }

    #[test]
    fn update_biome() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy(WITH_EXAMPLE_TERRAIN_TOML, &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for("example_biome", central_dir.path())
            .compile_terrain_script_for("none", central_dir.path())
            .successfully();

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(expected_shell_operation),
        );

        create_dir_all(context.scripts_dir()).expect("test scripts dir to be created");

        super::handle(
            context,
            Terrain::example(),
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

        AssertTerrain::with_dirs_and_existing(
            current_dir.path(),
            central_dir.path(),
            WITH_EXAMPLE_TERRAIN_TOML,
        )
        .was_updated(IN_CURRENT_DIR, WITH_EXAMPLE_BIOME_UPDATED_EXAMPLE_TOML)
        .script_was_created_for("none")
        .script_was_created_for("example_biome");
    }

    #[test]
    fn update_main() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy(WITH_EXAMPLE_TERRAIN_TOML, &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for("example_biome", central_dir.path())
            .compile_terrain_script_for("none", central_dir.path())
            .successfully();

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(expected_shell_operation),
        );

        create_dir_all(context.scripts_dir()).expect("test scripts dir to be created");

        super::handle(
            context,
            Terrain::example(),
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

        AssertTerrain::with_dirs_and_existing(
            current_dir.path(),
            central_dir.path(),
            WITH_EXAMPLE_TERRAIN_TOML,
        )
        .was_updated(IN_CURRENT_DIR, WITH_NONE_UPDATED_EXAMPLE_TOML)
        .script_was_created_for("none")
        .script_was_created_for("example_biome");
    }

    #[test]
    fn update_biome_invalid() {
        let current_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy(WITH_EXAMPLE_TERRAIN_TOML, &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(MockCommandToRun::default()),
        );

        let err = super::handle(
            context,
            Terrain::example(),
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

        AssertTerrain::with_dirs_and_existing(
            current_dir.path(),
            Path::new(""),
            WITH_EXAMPLE_TERRAIN_TOML,
        )
        .was_not_updated(IN_CURRENT_DIR);
    }

    #[test]
    fn backup() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy(WITH_EXAMPLE_TERRAIN_TOML, &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for("example_biome", central_dir.path())
            .compile_terrain_script_for("none", central_dir.path())
            .successfully();

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(expected_shell_operation),
        );

        create_dir_all(context.scripts_dir()).expect("test scripts dir to be created");

        super::handle(
            context,
            Terrain::example(),
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

        AssertTerrain::with_dirs_and_existing(
            current_dir.path(),
            central_dir.path(),
            WITH_EXAMPLE_TERRAIN_TOML,
        )
        .was_updated(IN_CURRENT_DIR, WITH_EXAMPLE_BIOME_UPDATED_EXAMPLE_TOML)
        .with_backup(IN_CURRENT_DIR)
        .script_was_created_for("none")
        .script_was_created_for("example_biome");
    }

    #[test]
    fn auto_apply() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy(WITH_EXAMPLE_TERRAIN_TOML, &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for("example_biome", central_dir.path())
            .compile_terrain_script_for("none", central_dir.path())
            .successfully();

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(expected_shell_operation),
        );

        create_dir_all(context.scripts_dir()).expect("test scripts dir to be created");

        super::handle(
            context,
            Terrain::example(),
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

        AssertTerrain::with_dirs_and_existing(
            current_dir.path(),
            central_dir.path(),
            WITH_EXAMPLE_TERRAIN_TOML,
        )
        .was_updated(IN_CURRENT_DIR, WITH_AUTO_APPLY_ENABLED_EXAMPLE_TOML)
        .with_backup(IN_CURRENT_DIR)
        .script_was_created_for("none")
        .script_was_created_for("example_biome");
    }

    #[test]
    fn auto_apply_turn_off() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy(WITH_AUTO_APPLY_ENABLED_EXAMPLE_TOML, &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for("example_biome", central_dir.path())
            .compile_terrain_script_for("none", central_dir.path())
            .successfully();

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(expected_shell_operation),
        );

        create_dir_all(context.scripts_dir()).expect("test scripts dir to be created");

        let mut terrain = Terrain::example();
        set_auto_apply(&mut terrain, "enable");

        super::handle(
            context,
            terrain,
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

        AssertTerrain::with_dirs_and_existing(
            current_dir.path(),
            central_dir.path(),
            WITH_AUTO_APPLY_ENABLED_EXAMPLE_TOML,
        )
        .was_updated(IN_CURRENT_DIR, WITH_EXAMPLE_TERRAIN_TOML)
        .with_backup(IN_CURRENT_DIR)
        .script_was_created_for("none")
        .script_was_created_for("example_biome");
    }
}
