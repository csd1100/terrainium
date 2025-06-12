use crate::client::handlers::edit;
use crate::client::shell::Shell;
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use anyhow::{Context as AnyhowContext, Result};
use std::fs::File;
use std::fs::{create_dir_all, exists};
use std::io::Write;

pub fn handle(context: Context, example: bool, edit: bool) -> Result<()> {
    if !exists(context.scripts_dir()).context("failed to check if scripts dir exists")? {
        create_dir_all(context.scripts_dir()).context("failed to create scripts dir")?;
    }

    let toml_path = context.toml_path();

    let mut file = File::create_new(toml_path).context("error while creating new terrain.toml")?;

    let mut terrain = if example {
        Terrain::example()
    } else {
        Terrain::default()
    };

    let toml_str = terrain
        .to_toml(context.terrain_dir())
        .expect("default or example terrain to be parsed to toml");

    file.write(toml_str.as_ref())
        .context("failed to write terrain in toml file")?;

    if edit {
        edit::run_editor(context.executor(), toml_path, context.terrain_dir())?;
        // get updated terrain after edit
        (terrain, _) = Terrain::get_validated_and_fixed_terrain(&context)?;
    }

    context.shell().generate_scripts(&context, terrain)?;

    Ok(())
}

#[cfg(test)]
pub mod tests {
    use crate::client::test_utils::assertions::executor::{AssertExecutor, ExpectedCommand};
    use crate::client::test_utils::assertions::terrain::AssertTerrain;
    use crate::client::test_utils::assertions::zsh::ExpectZSH;
    use crate::client::test_utils::constants::{
        IN_CENTRAL_DIR, IN_CURRENT_DIR, WITH_EMPTY_TERRAIN_TOML, WITH_EXAMPLE_TERRAIN_TOML,
    };
    use crate::client::types::config::Config;
    use crate::client::types::context::Context;
    use crate::common::constants::{EXAMPLE_BIOME, NONE};
    use crate::common::execute::MockExecutor;
    use crate::common::types::command::Command;
    use anyhow::Result;
    use serial_test::serial;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn init_creates_terrain_toml_in_current_dir() -> Result<()> {
        // setup
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let executor = ExpectZSH::with(MockExecutor::new(), current_dir.path().to_path_buf())
            .compile_terrain_script_for(NONE, central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Config::default(),
            executor,
        );

        // execute
        super::handle(context, false, false)?;

        // assertions
        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
            .was_initialized(IN_CURRENT_DIR, WITH_EMPTY_TERRAIN_TOML)
            .script_was_created_for(NONE);

        Ok(())
    }

    #[test]
    fn init_creates_terrain_toml_in_central_dir() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        // setup mock to assert scripts are compiled when init
        let executor = ExpectZSH::with(MockExecutor::new(), current_dir.path().to_path_buf())
            .compile_terrain_script_for(NONE, central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            central_dir.path().join("terrain.toml"),
            Config::default(),
            executor,
        );

        // execute
        super::handle(context, false, false)?;

        // assertions
        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
            .was_initialized(IN_CENTRAL_DIR, WITH_EMPTY_TERRAIN_TOML)
            .script_was_created_for(NONE);

        Ok(())
    }

    #[test]
    fn init_creates_scripts_dir_if_not_present() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        // setup mock to assert scripts are compiled when init
        let executor = ExpectZSH::with(MockExecutor::new(), current_dir.path().to_path_buf())
            .compile_terrain_script_for(NONE, central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Config::default(),
            executor,
        );

        super::handle(context, false, false)
            .expect("no error to be thrown when directory is not present");

        AssertTerrain::with_dirs(current_dir.path(), central_dir.path()).scripts_dir_was_created();

        Ok(())
    }

    #[test]
    fn init_creates_central_dir_if_not_present() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        // setup mock to assert scripts are compiled when init
        let executor = ExpectZSH::with(MockExecutor::new(), current_dir.path().to_path_buf())
            .compile_terrain_script_for(NONE, central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            central_dir.path().join("terrain.toml"),
            Config::default(),
            executor,
        );

        fs::remove_dir(&central_dir).expect("temp directory to be removed");

        super::handle(context, false, false)
            .expect("no error to be thrown when directory is not present");

        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
            .central_dir_is_created()
            .was_initialized(IN_CENTRAL_DIR, WITH_EMPTY_TERRAIN_TOML);

        Ok(())
    }

    #[test]
    fn init_throws_error_if_terrain_toml_exists_in_current_dir() -> Result<()> {
        let current_dir = tempdir()?;

        let context: Context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            current_dir.path().join("terrain.toml"),
            Config::default(),
            MockExecutor::new(),
        );

        let terrain_toml_path: PathBuf = current_dir.path().join("terrain.toml");

        fs::write(terrain_toml_path, "")?;

        let err = super::handle(context, false, false).expect_err("expected error to be thrown");

        assert_eq!(err.to_string(), "error while creating new terrain.toml");

        current_dir
            .close()
            .expect("expected directory to be cleaned up");

        Ok(())
    }

    #[test]
    fn init_creates_example_terrain_toml_in_central_dir() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        // setup mock to assert scripts are compiled when init
        let executor = ExpectZSH::with(MockExecutor::new(), current_dir.path().to_path_buf())
            .compile_terrain_script_for(EXAMPLE_BIOME, central_dir.path())
            .compile_terrain_script_for(NONE, central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            central_dir.path().join("terrain.toml"),
            Config::default(),
            executor,
        );

        super::handle(context, true, false)?;

        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
            .was_initialized(IN_CENTRAL_DIR, WITH_EXAMPLE_TERRAIN_TOML)
            .script_was_created_for(NONE)
            .script_was_created_for(EXAMPLE_BIOME);

        Ok(())
    }

    #[test]
    fn init_throws_error_if_terrain_toml_exists_in_central_dir() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            central_dir.path().join("terrain.toml"),
            Config::default(),
            MockExecutor::new(),
        );

        let terrain_toml_path: PathBuf = central_dir.path().join("terrain.toml");

        fs::write(terrain_toml_path, "")?;

        let err = super::handle(context, false, false).expect_err("expected error to be thrown");

        assert_eq!(err.to_string(), "error while creating new terrain.toml");

        Ok(())
    }

    #[test]
    fn init_creates_example_terrain_toml_in_current_dir() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        // setup mock to assert scripts are compiled when init
        let executor = ExpectZSH::with(MockExecutor::new(), current_dir.path().to_path_buf())
            .compile_terrain_script_for(EXAMPLE_BIOME, central_dir.path())
            .compile_terrain_script_for(NONE, central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Config::default(),
            executor,
        );

        super::handle(context, true, false)?;

        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
            .was_initialized(IN_CURRENT_DIR, WITH_EXAMPLE_TERRAIN_TOML)
            .script_was_created_for(NONE)
            .script_was_created_for(EXAMPLE_BIOME);

        Ok(())
    }

    #[serial]
    #[test]
    fn init_creates_and_edits_terrain_toml_in_current_dir() -> Result<()> {
        let editor = std::env::var("EDITOR")?;

        // setup
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        //setup edit mock
        let terrain_toml_path: PathBuf = current_dir.path().join("terrain.toml");
        let terrain_dir = current_dir.path().to_path_buf();

        let expected = ExpectedCommand {
            command: Command::new(
                editor,
                vec![terrain_toml_path.to_string_lossy().to_string()],
                None,
                Some(terrain_dir.clone()),
            ),
            exit_code: 0,
            should_error: false,
            output: String::new(),
        };

        let executor = AssertExecutor::to().wait_for(expected);

        // setup mock to assert scripts are compiled when init
        let executor = ExpectZSH::with(executor, current_dir.path().to_path_buf())
            .compile_terrain_script_for(NONE, central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Config::default(),
            executor,
        );

        // execute
        super::handle(context, false, true)?;

        // assertions
        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
            .was_initialized(IN_CURRENT_DIR, WITH_EMPTY_TERRAIN_TOML)
            .script_was_created_for(NONE);

        Ok(())
    }

    #[serial]
    #[test]
    fn init_creates_and_edits_terrain_toml_in_central_dir() -> Result<()> {
        let editor = std::env::var("EDITOR")?;

        // setup
        let terrain_dir = tempdir()?;
        let central_dir = tempdir()?;

        //setup edit mock
        let terrain_toml_path: PathBuf = central_dir.path().join("terrain.toml");
        let terrain_dir_path = terrain_dir.path().to_path_buf();

        let expected = ExpectedCommand {
            command: Command::new(
                editor,
                vec![terrain_toml_path.to_string_lossy().to_string()],
                None,
                Some(terrain_dir_path.clone()),
            ),
            exit_code: 0,
            should_error: false,
            output: String::new(),
        };

        let executor = AssertExecutor::to().wait_for(expected);

        // setup mock to assert scripts are compiled when init
        let executor = ExpectZSH::with(executor, terrain_dir.path().to_path_buf())
            .compile_terrain_script_for(NONE, central_dir.path())
            .successfully();

        let context: Context = Context::build(
            terrain_dir.path().into(),
            central_dir.path().into(),
            central_dir.path().join("terrain.toml"),
            Config::default(),
            executor,
        );

        // execute
        super::handle(context, false, true)?;

        // assertions
        AssertTerrain::with_dirs(terrain_dir.path(), central_dir.path())
            .was_initialized(IN_CENTRAL_DIR, WITH_EMPTY_TERRAIN_TOML)
            .script_was_created_for(NONE);

        Ok(())
    }
}
