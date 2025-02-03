use crate::client::handlers::edit;
use crate::client::shell::Shell;
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use anyhow::{anyhow, Context as AnyhowContext, Result};
use std::fs;
use std::fs::File;
use std::io::Write;

pub fn handle(context: Context, central: bool, example: bool, edit: bool) -> Result<()> {
    if context.toml_exists() {
        return Err(anyhow!("terrain for this project is already present. edit existing terrain with `terrain edit` command"));
    }

    if !fs::exists(context.scripts_dir()).context("failed to check if scripts dir exists")? {
        fs::create_dir_all(context.scripts_dir()).context("failed to create scripts dir")?;
    }

    let toml_path = context.new_toml_path(central);

    let mut file = File::create_new(&toml_path).context("error while creating terrain.toml")?;

    let terrain = if example {
        Terrain::example()
    } else {
        Terrain::default()
    };

    let toml_str = terrain
        .to_toml(context.terrain_dir())
        .expect("terrain to be parsed to toml");

    file.write(toml_str.as_ref())
        .context("failed to write terrain in toml file")?;

    if edit {
        // TODO: get validated toml from run_editor
        edit::run_editor(&toml_path, context.terrain_dir())?;
    }

    // TODO: fix validations here
    // FIXME: if edited run generate_scripts with updated terrain
    context.shell().generate_scripts(&context, terrain)?;

    Ok(())
}

#[cfg(test)]
pub mod tests {
    use crate::client::handlers::edit::tests::EDITOR;
    use crate::client::shell::Zsh;
    use crate::client::types::context::Context;
    use crate::client::utils::{
        restore_env_var, set_env_var, AssertTerrain, ExpectShell, IN_CENTRAL_DIR, IN_CURRENT_DIR,
        WITH_EMPTY_TERRAIN_TOML, WITH_EXAMPLE_TERRAIN_TOML,
    };
    use crate::common::execute::MockCommandToRun;
    use anyhow::Result;
    use serial_test::serial;
    use std::fs;
    use std::os::unix::prelude::ExitStatusExt;
    use std::path::PathBuf;
    use std::process::ExitStatus;
    use tempfile::tempdir;

    #[test]
    fn init_creates_terrain_toml_in_current_dir() -> Result<()> {
        // setup
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for("none", central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        // execute
        super::handle(context, false, false, false)?;

        // assertions
        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
            .was_initialized(IN_CURRENT_DIR, WITH_EMPTY_TERRAIN_TOML)
            .script_was_created_for("none");

        Ok(())
    }

    #[test]
    fn init_creates_terrain_toml_in_central_dir() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        // setup mock to assert scripts are compiled when init
        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for("none", central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        // execute
        super::handle(context, true, false, false)?;

        // assertions
        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
            .was_initialized(IN_CENTRAL_DIR, WITH_EMPTY_TERRAIN_TOML)
            .script_was_created_for("none");

        Ok(())
    }

    #[test]
    fn init_creates_scripts_dir_if_not_present() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        // setup mock to assert scripts are compiled when init
        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for("none", central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        super::handle(context, false, false, false)
            .expect("no error to be thrown when directory is not present");

        AssertTerrain::with_dirs(current_dir.path(), central_dir.path()).scripts_dir_was_created();

        Ok(())
    }

    #[test]
    fn init_creates_central_dir_if_not_present() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        // setup mock to assert scripts are compiled when init
        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for("none", central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        fs::remove_dir(&central_dir).expect("temp directory to be removed");

        super::handle(context, true, false, false)
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
            Zsh::build(MockCommandToRun::default()),
        );

        let terrain_toml_path: PathBuf = current_dir.path().join("terrain.toml");

        fs::write(terrain_toml_path, "")?;

        let err =
            super::handle(context, false, false, false).expect_err("expected error to be thrown");

        assert_eq!(err.to_string(), "terrain for this project is already present. edit existing terrain with `terrain edit` command");

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
        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for("example_biome", central_dir.path())
            .compile_terrain_script_for("none", central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        super::handle(context, true, true, false)?;

        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
            .was_initialized(IN_CENTRAL_DIR, WITH_EXAMPLE_TERRAIN_TOML)
            .script_was_created_for("none")
            .script_was_created_for("example_biome");

        Ok(())
    }

    #[test]
    fn init_throws_error_if_terrain_toml_exists_in_central_dir() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(MockCommandToRun::default()),
        );

        let terrain_toml_path: PathBuf = central_dir.path().join("terrain.toml");

        fs::write(terrain_toml_path, "")?;

        let err =
            super::handle(context, true, false, false).expect_err("expected error to be thrown");

        assert_eq!(err.to_string(), "terrain for this project is already present. edit existing terrain with `terrain edit` command");

        Ok(())
    }

    #[test]
    fn init_creates_example_terrain_toml_in_current_dir() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        // setup mock to assert scripts are compiled when init
        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for("example_biome", central_dir.path())
            .compile_terrain_script_for("none", central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        super::handle(context, false, true, false)?;

        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
            .was_initialized(IN_CURRENT_DIR, WITH_EXAMPLE_TERRAIN_TOML)
            .script_was_created_for("none")
            .script_was_created_for("example_biome");

        Ok(())
    }

    #[serial]
    #[test]
    fn init_creates_and_edits_terrain_toml_in_current_dir() -> Result<()> {
        let editor = set_env_var(EDITOR.to_string(), Some("vim".to_string()));

        // setup
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        //setup edit mock
        let terrain_toml_path: PathBuf = current_dir.path().join("terrain.toml");
        let terrain_dir = current_dir.path().to_path_buf();

        let mut edit_run = MockCommandToRun::default();
        edit_run
            .expect_wait()
            .with()
            .times(1)
            .return_once(|| Ok(ExitStatus::from_raw(0)));
        let edit_mock = MockCommandToRun::new_context();
        edit_mock
            .expect()
            .withf(move |exe, args, envs, cwd| {
                exe == &"vim".to_string()
                    && *args == vec![terrain_toml_path.to_string_lossy()]
                    && envs.is_some()
                    && *cwd == terrain_dir
            })
            .times(1)
            .return_once(|_, _, _, _| edit_run);

        // setup mock to assert scripts are compiled when init
        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for("none", central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        // execute
        super::handle(context, false, false, true)?;

        // assertions
        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
            .was_initialized(IN_CURRENT_DIR, WITH_EMPTY_TERRAIN_TOML)
            .script_was_created_for("none");

        restore_env_var(EDITOR.to_string(), editor);
        Ok(())
    }
}
