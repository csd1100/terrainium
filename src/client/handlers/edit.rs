use crate::client::shell::Shell;
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use crate::common::execute::Execute;
#[mockall_double::double]
use crate::common::execute::Executor;
use crate::common::types::command::Command;
use anyhow::{Context as AnyhowContext, Result};
use std::path::Path;
use std::sync::Arc;
use tracing::info;

const EDITOR: &str = "EDITOR";

pub fn handle(context: Context) -> Result<()> {
    run_editor(
        context.executor(),
        context.toml_path(),
        context.terrain_dir(),
    )?;

    let (terrain, _) = Terrain::get_validated_and_fixed_terrain(&context)?;
    context.shell().generate_scripts(&context, terrain)?;

    Ok(())
}

pub(crate) fn run_editor(
    executor: &Arc<Executor>,
    toml_path: &Path,
    terrain_dir: &Path,
) -> Result<()> {
    let editor = std::env::var(EDITOR).unwrap_or_else(|_| {
        info!("the environment variable EDITOR not set. using 'vi' as text editor");
        "vi".to_string()
    });

    let command = Command::new(
        editor,
        vec![
            toml_path
                .to_string_lossy()
                .parse()
                .context(format!("failed to convert path {:?} to string", toml_path))?,
        ],
        Some(terrain_dir.to_path_buf()),
    );

    executor
        .wait(None, command, false)
        .context(format!("failed to edit file {:?}", toml_path))?;

    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::client::test_utils::assertions::executor::{AssertExecutor, ExpectedCommand};
    use crate::client::test_utils::assertions::terrain::AssertTerrain;
    use crate::client::test_utils::assertions::zsh::ExpectZSH;
    use crate::client::test_utils::constants::{
        IN_CENTRAL_DIR, IN_CURRENT_DIR, WITH_EXAMPLE_TERRAIN_TOML,
    };
    use crate::client::test_utils::{restore_env_var, set_env_var};
    use crate::client::types::context::Context;
    use crate::common::constants::{EXAMPLE_BIOME, NONE, TERRAIN_TOML};
    use crate::common::types::command::Command;
    use anyhow::Result;
    use fs::{copy, create_dir_all};
    use serial_test::serial;
    use std::env::VarError;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    pub(crate) const EDITOR: &str = "EDITOR";

    #[serial]
    #[test]
    fn edit_opens_editor_and_generates_scripts_current_dir() -> Result<()> {
        let editor: std::result::Result<String, VarError>;
        unsafe {
            editor = set_env_var(EDITOR, Some("vim"));
        }

        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let terrain_toml: PathBuf = current_dir.path().join(TERRAIN_TOML);
        let terrain_dir = current_dir.path();

        let expected = ExpectedCommand {
            command: Command::new(
                "vim".to_string(),
                vec![terrain_toml.to_string_lossy().to_string()],
                Some(terrain_dir.into()),
            ),
            exit_code: 0,
            should_fail_to_execute: false,
            output: String::new(),
        };

        let executor = AssertExecutor::to().wait_for(None, expected, false, 1);

        // setup mock to assert scripts are compiled when edit
        let executor = ExpectZSH::with(executor, terrain_dir)
            .compile_terrain_script_for(EXAMPLE_BIOME, central_dir.path())
            .compile_terrain_script_for(NONE, central_dir.path())
            .successfully();

        let context: Context =
            Context::build(current_dir.path(), central_dir.path(), false, executor);

        let terrain_toml: PathBuf = current_dir.path().join(TERRAIN_TOML);
        copy("./tests/data/terrain.example.toml", terrain_toml).expect("test file to be copied");

        let script_dir = central_dir.path().join("scripts");
        create_dir_all(script_dir).expect("test scripts dir to be created");

        super::handle(context).expect("no error to be thrown");

        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
            .was_initialized(IN_CURRENT_DIR, WITH_EXAMPLE_TERRAIN_TOML)
            .script_was_created_for(NONE)
            .script_was_created_for(EXAMPLE_BIOME);

        unsafe {
            restore_env_var(EDITOR, editor);
        }
        Ok(())
    }

    #[serial]
    #[test]
    fn edit_opens_editor_and_generates_scripts_central_dir() -> Result<()> {
        let editor: std::result::Result<String, VarError>;
        unsafe {
            editor = set_env_var(EDITOR, Some("vim"));
        }

        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let terrain_toml: PathBuf = central_dir.path().join(TERRAIN_TOML);
        let terrain_dir = current_dir.path();

        let expected = ExpectedCommand {
            command: Command::new(
                "vim".to_string(),
                vec![terrain_toml.to_string_lossy().to_string()],
                Some(terrain_dir.into()),
            ),
            exit_code: 0,
            should_fail_to_execute: false,
            output: String::new(),
        };

        let executor = AssertExecutor::to().wait_for(None, expected, false, 1);

        // setup mock to assert scripts are compiled when edit
        let executor = ExpectZSH::with(executor, terrain_dir)
            .compile_terrain_script_for(EXAMPLE_BIOME, central_dir.path())
            .compile_terrain_script_for(NONE, central_dir.path())
            .successfully();

        let context: Context =
            Context::build(current_dir.path(), central_dir.path(), true, executor);

        let terrain_toml: PathBuf = central_dir.path().join(TERRAIN_TOML);
        copy("./tests/data/terrain.example.toml", terrain_toml).expect("test file to be copied");

        let central_dir1 = central_dir.path();
        let script_dir = central_dir1.join("scripts");
        create_dir_all(script_dir).expect("test scripts dir to be created");

        super::handle(context).expect("no error to be thrown");

        // assert example_biome script is created
        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
            .was_initialized(IN_CENTRAL_DIR, WITH_EXAMPLE_TERRAIN_TOML)
            .script_was_created_for(NONE)
            .script_was_created_for(EXAMPLE_BIOME);

        unsafe {
            restore_env_var(EDITOR, editor);
        }
        Ok(())
    }

    #[serial]
    #[test]
    fn edit_opens_default_editor_if_env_not_set_and_generates_scripts() -> Result<()> {
        let editor: std::result::Result<String, VarError>;
        unsafe {
            editor = set_env_var(EDITOR, None);
        }

        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let terrain_toml: PathBuf = current_dir.path().join(TERRAIN_TOML);
        let terrain_dir = current_dir.path();

        let expected = ExpectedCommand {
            command: Command::new(
                "vi".to_string(),
                vec![terrain_toml.to_string_lossy().to_string()],
                Some(terrain_dir.into()),
            ),
            exit_code: 0,
            should_fail_to_execute: false,
            output: String::new(),
        };

        let executor = AssertExecutor::to().wait_for(None, expected, false, 1);

        // setup mock to assert scripts are compiled when edit
        let executor = ExpectZSH::with(executor, terrain_dir)
            .compile_terrain_script_for(EXAMPLE_BIOME, central_dir.path())
            .compile_terrain_script_for(NONE, central_dir.path())
            .successfully();

        let context: Context =
            Context::build(current_dir.path(), central_dir.path(), false, executor);

        let terrain_toml: PathBuf = current_dir.path().join(TERRAIN_TOML);
        copy("./tests/data/terrain.example.toml", terrain_toml).expect("test file to be copied");

        let central_dir1 = central_dir.path();
        let script_dir = central_dir1.join("scripts");
        create_dir_all(script_dir).expect("test scripts dir to be created");

        super::handle(context).expect("no error to be thrown");

        // assert example_biome script is created
        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
            .was_initialized(IN_CURRENT_DIR, WITH_EXAMPLE_TERRAIN_TOML)
            .script_was_created_for(NONE)
            .script_was_created_for(EXAMPLE_BIOME);

        unsafe {
            restore_env_var(EDITOR, editor);
        }
        Ok(())
    }
}
