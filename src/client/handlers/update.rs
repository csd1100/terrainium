use crate::client::args::UpdateArgs;
use crate::client::shell::Shell;
use crate::client::types::biome::Biome;
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use crate::common::constants::{
    ALIASES, AUTO_APPLY, AUTO_APPLY_BACKGROUND, AUTO_APPLY_ENABLED, AUTO_APPLY_REPLACE, BIOMES,
    DEFAULT_BIOME, ENVS, NONE, TERRAIN,
};
use anyhow::{anyhow, Context as AnyhowContext, Result};
use std::fs::{copy, write};
use toml_edit::{value, DocumentMut};

pub fn handle(
    context: Context,
    terrain: Terrain,
    mut terrain_toml: DocumentMut,
    update_args: UpdateArgs,
) -> Result<()> {
    if let Some(auto_apply) = update_args.auto_apply {
        terrain_toml[AUTO_APPLY][AUTO_APPLY_ENABLED] = value(auto_apply.get_enabled());
        terrain_toml[AUTO_APPLY][AUTO_APPLY_BACKGROUND] = value(auto_apply.get_background());
        terrain_toml[AUTO_APPLY][AUTO_APPLY_REPLACE] = value(auto_apply.get_replace());
    }

    if let Some(new_default) = update_args.set_default {
        if !terrain.biomes().contains_key(&new_default) {
            return Err(anyhow!(
                "cannot update default biome to '{new_default}', biome '{new_default}' does not exists",
            ));
        }
        terrain_toml[DEFAULT_BIOME] = value(new_default);
    } else {
        let biome_name = if update_args.new.is_some() {
            let new_biome = update_args.new.expect("new biome to be some");
            terrain_toml[BIOMES][&new_biome] = Biome::new_toml().into();
            new_biome
        } else {
            terrain.select_biome(&update_args.biome)?.name()
        };

        update_args.env.into_iter().for_each(|env| {
            if biome_name == NONE {
                terrain_toml[TERRAIN][ENVS][env.key] = value(env.value);
            } else {
                terrain_toml[BIOMES][&biome_name][ENVS][env.key] = value(env.value);
            }
        });

        update_args.alias.into_iter().for_each(|alias| {
            if biome_name == NONE {
                terrain_toml[TERRAIN][ALIASES][alias.key] = value(alias.value);
            } else {
                terrain_toml[BIOMES][&biome_name][ALIASES][alias.key] = value(alias.value);
            }
        });
    }

    if update_args.backup {
        let mut backup = context.toml_path().to_path_buf();
        backup.set_extension("toml.bkp");

        copy(context.toml_path(), backup).context("failed to backup terrain.toml")?;
    }

    write(context.toml_path(), terrain_toml.to_string()).context("failed to write updated toml")?;
    let (validated_and_fixed, _) = Terrain::get_validated_and_fixed_terrain(&context)?;

    context
        .shell()
        .generate_scripts(&context, validated_and_fixed)?;

    Ok(())
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
        WITH_EXAMPLE_TERRAIN_TOML_COMMENTS, WITH_NEW_EXAMPLE_BIOME2_EXAMPLE_TOML,
        WITH_NONE_UPDATED_EXAMPLE_TOML,
    };
    use crate::common::constants::{EXAMPLE_BIOME, NONE};
    use crate::common::execute::MockCommandToRun;
    use std::fs::{copy, create_dir_all, read_to_string};
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;
    use toml_edit::DocumentMut;

    #[test]
    fn set_default_biome() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
        copy(WITHOUT_DEFAULT_BIOME_TOML, &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let toml = read_to_string(&terrain_toml)
            .unwrap()
            .parse::<DocumentMut>()
            .unwrap();

        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for(EXAMPLE_BIOME, central_dir.path())
            .compile_terrain_script_for(NONE, central_dir.path())
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
            toml,
            UpdateArgs {
                set_default: Some(EXAMPLE_BIOME.to_string()),
                biome: BiomeArg::Default,
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
        .was_updated(IN_CURRENT_DIR, WITH_EXAMPLE_TERRAIN_TOML_COMMENTS)
        .script_was_created_for(NONE)
        .script_was_created_for(EXAMPLE_BIOME);
    }

    #[test]
    fn set_default_biome_invalid() {
        let current_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
        copy(WITHOUT_DEFAULT_BIOME_TOML, &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let toml = read_to_string(&terrain_toml)
            .unwrap()
            .parse::<DocumentMut>()
            .unwrap();

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
            toml,
            UpdateArgs {
                set_default: Some("non_existent".to_string()),
                biome: BiomeArg::Default,
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

        copy(WITH_EXAMPLE_TERRAIN_TOML_COMMENTS, &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let toml = read_to_string(&terrain_toml)
            .unwrap()
            .parse::<DocumentMut>()
            .unwrap();

        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for(EXAMPLE_BIOME, central_dir.path())
            .compile_terrain_script_for("example_biome2", central_dir.path())
            .compile_terrain_script_for(NONE, central_dir.path())
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
            toml,
            UpdateArgs {
                set_default: None,
                biome: BiomeArg::Default,
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
            WITH_EXAMPLE_TERRAIN_TOML_COMMENTS,
        )
        .was_updated(IN_CURRENT_DIR, WITH_NEW_EXAMPLE_BIOME2_EXAMPLE_TOML)
        .script_was_created_for(NONE)
        .script_was_created_for(EXAMPLE_BIOME)
        .script_was_created_for("example_biome2");
    }

    #[test]
    fn update_biome() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy(WITH_EXAMPLE_TERRAIN_TOML_COMMENTS, &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let toml = read_to_string(&terrain_toml)
            .unwrap()
            .parse::<DocumentMut>()
            .unwrap();

        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for(EXAMPLE_BIOME, central_dir.path())
            .compile_terrain_script_for(NONE, central_dir.path())
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
            toml,
            UpdateArgs {
                set_default: None,
                biome: BiomeArg::Default,
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
            WITH_EXAMPLE_TERRAIN_TOML_COMMENTS,
        )
        .was_updated(IN_CURRENT_DIR, WITH_EXAMPLE_BIOME_UPDATED_EXAMPLE_TOML)
        .script_was_created_for(NONE)
        .script_was_created_for(EXAMPLE_BIOME);
    }

    #[test]
    fn update_main() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy(WITH_EXAMPLE_TERRAIN_TOML_COMMENTS, &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let toml = read_to_string(&terrain_toml)
            .unwrap()
            .parse::<DocumentMut>()
            .unwrap();

        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for(EXAMPLE_BIOME, central_dir.path())
            .compile_terrain_script_for(NONE, central_dir.path())
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
            toml,
            UpdateArgs {
                set_default: None,
                biome: BiomeArg::None,
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
            WITH_EXAMPLE_TERRAIN_TOML_COMMENTS,
        )
        .was_updated(IN_CURRENT_DIR, WITH_NONE_UPDATED_EXAMPLE_TOML)
        .script_was_created_for(NONE)
        .script_was_created_for(EXAMPLE_BIOME);
    }

    #[test]
    fn update_biome_invalid() {
        let current_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy(WITH_EXAMPLE_TERRAIN_TOML_COMMENTS, &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let toml = read_to_string(&terrain_toml)
            .unwrap()
            .parse::<DocumentMut>()
            .unwrap();

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(MockCommandToRun::default()),
        );

        let err = super::handle(
            context,
            Terrain::example(),
            toml,
            UpdateArgs {
                set_default: None,
                biome: BiomeArg::Some("non_existent".to_string()),
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
            WITH_EXAMPLE_TERRAIN_TOML_COMMENTS,
        )
        .was_not_updated(IN_CURRENT_DIR);
    }

    #[test]
    fn backup() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy(WITH_EXAMPLE_TERRAIN_TOML_COMMENTS, &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let toml = read_to_string(&terrain_toml)
            .unwrap()
            .parse::<DocumentMut>()
            .unwrap();

        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for(EXAMPLE_BIOME, central_dir.path())
            .compile_terrain_script_for(NONE, central_dir.path())
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
            toml,
            UpdateArgs {
                set_default: None,
                biome: BiomeArg::Default,
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
            WITH_EXAMPLE_TERRAIN_TOML_COMMENTS,
        )
        .was_updated(IN_CURRENT_DIR, WITH_EXAMPLE_BIOME_UPDATED_EXAMPLE_TOML)
        .with_backup(IN_CURRENT_DIR)
        .script_was_created_for(NONE)
        .script_was_created_for(EXAMPLE_BIOME);
    }

    #[test]
    fn auto_apply() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy(WITH_EXAMPLE_TERRAIN_TOML_COMMENTS, &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let toml = read_to_string(&terrain_toml)
            .unwrap()
            .parse::<DocumentMut>()
            .unwrap();

        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for(EXAMPLE_BIOME, central_dir.path())
            .compile_terrain_script_for(NONE, central_dir.path())
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
            toml,
            UpdateArgs {
                set_default: None,
                biome: BiomeArg::Default,
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
            WITH_EXAMPLE_TERRAIN_TOML_COMMENTS,
        )
        .was_updated(IN_CURRENT_DIR, WITH_AUTO_APPLY_ENABLED_EXAMPLE_TOML)
        .with_backup(IN_CURRENT_DIR)
        .script_was_created_for(NONE)
        .script_was_created_for(EXAMPLE_BIOME);
    }

    #[test]
    fn auto_apply_turn_off() {
        let current_dir = tempdir().expect("tempdir to be created");
        let central_dir = tempdir().expect("tempdir to be created");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy(WITH_AUTO_APPLY_ENABLED_EXAMPLE_TOML, &terrain_toml)
            .expect("test terrain to be copied to test dir");

        let toml = read_to_string(&terrain_toml)
            .unwrap()
            .parse::<DocumentMut>()
            .unwrap();

        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for(EXAMPLE_BIOME, central_dir.path())
            .compile_terrain_script_for(NONE, central_dir.path())
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
            toml,
            UpdateArgs {
                set_default: None,
                biome: BiomeArg::Default,
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
        .was_updated(IN_CURRENT_DIR, WITH_EXAMPLE_TERRAIN_TOML_COMMENTS)
        .with_backup(IN_CURRENT_DIR)
        .script_was_created_for(NONE)
        .script_was_created_for(EXAMPLE_BIOME);
    }
}
