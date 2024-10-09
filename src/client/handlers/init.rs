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

    let toml_str = terrain.to_toml().expect("terrain to be parsed to toml");

    file.write(toml_str.as_ref())
        .context("failed to write terrain in toml file")?;

    if edit {
        edit::run_editor(&toml_path)?;
    }

    context.shell().generate_scripts(&context, terrain)?;

    Ok(())
}

#[cfg(test)]
pub mod test {
    use crate::client::handlers::edit::test::EDITOR;
    use crate::client::old_utils::test::script_path;
    use crate::client::shell::Zsh;
    use crate::client::types::context::Context;
    use crate::client::utils::ExpectShell;
    use crate::common::execute::test::{restore_env_var, set_env_var};
    use crate::common::execute::MockCommandToRun;
    use anyhow::Result;
    use serial_test::serial;
    use std::fs;
    use std::os::unix::process::ExitStatusExt;
    use std::path::PathBuf;
    use std::process::ExitStatus;
    use tempfile::tempdir;

    #[test]
    fn init_creates_terrain_toml_in_current_dir() -> Result<()> {
        // setup
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let expected_shell_operation = ExpectShell::to()
            .compile_script_for("none", central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        // execute
        super::handle(context, false, false, false)?;

        // assertions
        // assert terrain.toml is created
        let terrain_toml_path: PathBuf = current_dir.path().join("terrain.toml");
        assert!(
            fs::exists(&terrain_toml_path)?,
            "expected terrain.toml to be created in current directory"
        );

        let actual_terrain_toml =
            fs::read_to_string(&terrain_toml_path).expect("expected terrain.toml to be readable");
        let expected_terrain_toml = fs::read_to_string("./tests/data/terrain.empty.toml")
            .expect("expected test toml to be readable");

        assert_eq!(actual_terrain_toml, expected_terrain_toml);

        // assert scripts are created
        let script_path: PathBuf = script_path(central_dir.path(), &"none".to_string());
        assert!(
            fs::exists(&script_path)?,
            "expected terrain-none.zsh to be created in current directory"
        );

        let actual_script =
            fs::read_to_string(&script_path).expect("expected terrain-none.zsh to be readable");
        let expected_script = fs::read_to_string("./tests/data/terrain-none.empty.zsh")
            .expect("expected test script to be readable");

        assert_eq!(actual_script, expected_script);

        current_dir
            .close()
            .expect("expected directory to be cleaned up");

        Ok(())
    }

    #[test]
    fn init_creates_terrain_toml_in_central_dir() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        // setup mock to assert scripts are compiled when init
        let expected_shell_operation = ExpectShell::to()
            .compile_script_for("none", central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        // execute
        super::handle(context, true, false, false)?;

        // assertions
        // assert toml is created in central dir
        let terrain_toml_path: PathBuf = central_dir.path().join("terrain.toml");

        assert!(
            fs::exists(&terrain_toml_path)?,
            "expected terrain.toml to be created in central directory"
        );

        let actual =
            fs::read_to_string(&terrain_toml_path).expect("expected terrain.toml to be readable");
        let expected = fs::read_to_string("./tests/data/terrain.empty.toml")
            .expect("expected test toml to be readable");

        assert_eq!(actual, expected);

        // assert scripts are created
        let script_path: PathBuf = script_path(central_dir.path(), &"none".to_string());
        assert!(
            fs::exists(&script_path)?,
            "expected terrain-none.zsh to be created in current directory"
        );

        let actual_script =
            fs::read_to_string(&script_path).expect("expected terrain-none.zsh to be readable");
        let expected_script = fs::read_to_string("./tests/data/terrain-none.empty.zsh")
            .expect("expected test script to be readable");

        assert_eq!(actual_script, expected_script);

        current_dir
            .close()
            .expect("expected directory to be cleaned up");
        central_dir
            .close()
            .expect("expected directory to be cleaned up");

        Ok(())
    }

    #[test]
    fn init_creates_scripts_dir_if_not_present() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        // setup mock to assert scripts are compiled when init
        let expected_shell_operation = ExpectShell::to()
            .compile_script_for("none", central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        super::handle(context, false, false, false)
            .expect("no error to be thrown when directory is not present");

        let scripts_dir_path: PathBuf = central_dir.path().join("scripts");

        assert!(
            fs::exists(&scripts_dir_path)?,
            "expected terrain.toml to be created in central directory"
        );

        current_dir
            .close()
            .expect("expected directory to be cleaned up");
        central_dir
            .close()
            .expect("expected directory to be cleaned up");

        Ok(())
    }

    #[test]
    fn init_creates_central_dir_if_not_present() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        // setup mock to assert scripts are compiled when init
        let expected_shell_operation = ExpectShell::to()
            .compile_script_for("none", central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        fs::remove_dir(&central_dir).expect("temp directory to be removed");

        super::handle(context, true, false, false)
            .expect("no error to be thrown when directory is not present");

        let terrain_toml_path: PathBuf = central_dir.path().join("terrain.toml");

        assert!(
            fs::exists(&terrain_toml_path)?,
            "expected terrain.toml to be created in central directory"
        );

        let actual =
            fs::read_to_string(&terrain_toml_path).expect("expected terrain.toml to be readable");
        let expected = fs::read_to_string("./tests/data/terrain.empty.toml")
            .expect("expected test toml to be readable");

        assert_eq!(actual, expected);

        current_dir
            .close()
            .expect("expected directory to be cleaned up");
        central_dir
            .close()
            .expect("expected directory to be cleaned up");

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
            .compile_script_for("example_biome", central_dir.path())
            .compile_script_for("none", central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        super::handle(context, true, true, false)?;

        let terrain_toml_path: PathBuf = central_dir.path().join("terrain.toml");

        assert!(
            fs::exists(&terrain_toml_path)?,
            "expected terrain.toml to be created in current directory"
        );

        let actual =
            fs::read_to_string(&terrain_toml_path).expect("expected terrain.toml to be readable");
        let expected = fs::read_to_string("./tests/data/terrain.example.toml")
            .expect("expected test toml to be readable");

        assert_eq!(actual, expected);

        // assert example_biome script is created
        let script: PathBuf = script_path(central_dir.path(), &"example_biome".to_string());

        assert!(
            fs::exists(&script)?,
            "expected terrain-example_biome.zsh to be created in scripts directory"
        );

        let actual = fs::read_to_string(&script).expect("expected terrain.toml to be readable");
        let expected = fs::read_to_string("./tests/data/terrain-example_biome.example.zsh")
            .expect("expected test toml to be readable");

        assert_eq!(actual, expected);

        // assert none script is created
        let script_path: PathBuf = script_path(central_dir.path(), &"none".to_string());
        assert!(
            fs::exists(&script_path)?,
            "expected terrain-none.zsh to be created in current directory"
        );

        let actual_script =
            fs::read_to_string(&script_path).expect("expected terrain-none.zsh to be readable");
        let expected_script = fs::read_to_string("./tests/data/terrain-none.example.zsh")
            .expect("expected test script to be readable");

        assert_eq!(actual_script, expected_script);

        current_dir
            .close()
            .expect("expected directory to be cleaned up");
        central_dir
            .close()
            .expect("expected directory to be cleaned up");

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

        current_dir
            .close()
            .expect("expected directory to be cleaned up");
        central_dir
            .close()
            .expect("expected directory to be cleaned up");

        Ok(())
    }

    #[test]
    fn init_creates_example_terrain_toml_in_current_dir() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        // setup mock to assert scripts are compiled when init
        let expected_shell_operation = ExpectShell::to()
            .compile_script_for("example_biome", central_dir.path())
            .compile_script_for("none", central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        super::handle(context, false, true, false)?;

        // assert toml is created
        let terrain_toml_path: PathBuf = current_dir.path().join("terrain.toml");

        assert!(
            fs::exists(&terrain_toml_path)?,
            "expected terrain.toml to be created in current directory"
        );

        let actual =
            fs::read_to_string(&terrain_toml_path).expect("expected terrain.toml to be readable");
        let expected = fs::read_to_string("./tests/data/terrain.example.toml")
            .expect("expected test toml to be readable");

        assert_eq!(actual, expected);

        // assert example_biome script is created
        let script: PathBuf = script_path(central_dir.path(), &"example_biome".to_string());

        assert!(
            fs::exists(&script)?,
            "expected terrain-example_biome.zsh to be created in scripts directory"
        );

        let actual = fs::read_to_string(&script).expect("expected terrain.toml to be readable");
        let expected = fs::read_to_string("./tests/data/terrain-example_biome.example.zsh")
            .expect("expected test toml to be readable");

        assert_eq!(actual, expected);

        // assert none script is created
        let script_path: PathBuf = script_path(central_dir.path(), &"none".to_string());
        assert!(
            fs::exists(&script_path)?,
            "expected terrain-none.zsh to be created in current directory"
        );

        let actual_script =
            fs::read_to_string(&script_path).expect("expected terrain-none.zsh to be readable");
        let expected_script = fs::read_to_string("./tests/data/terrain-none.example.zsh")
            .expect("expected test script to be readable");

        assert_eq!(actual_script, expected_script);

        current_dir
            .close()
            .expect("expected directory to be cleaned up");

        central_dir
            .close()
            .expect("expected directory to be cleaned up");

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
        let mut edit_run = MockCommandToRun::default();
        edit_run
            .expect_wait()
            .with()
            .times(1)
            .return_once(|| Ok(ExitStatus::from_raw(0)));
        let edit_mock = MockCommandToRun::new_context();
        edit_mock
            .expect()
            .withf(move |exe, args, envs| {
                exe == &"vim".to_string()
                    && *args == vec![terrain_toml_path.to_string_lossy()]
                    && envs.is_some()
            })
            .times(1)
            .return_once(|_, _, _| edit_run);

        // setup mock to assert scripts are compiled when init
        let expected_shell_operation = ExpectShell::to()
            .compile_script_for("none", central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        // execute
        super::handle(context, false, false, true)?;

        // assertions
        // assert terrain.toml is created
        let terrain_toml_path: PathBuf = current_dir.path().join("terrain.toml");
        assert!(
            fs::exists(&terrain_toml_path)?,
            "expected terrain.toml to be created in current directory"
        );

        let actual_terrain_toml =
            fs::read_to_string(&terrain_toml_path).expect("expected terrain.toml to be readable");
        let expected_terrain_toml = fs::read_to_string("./tests/data/terrain.empty.toml")
            .expect("expected test toml to be readable");

        assert_eq!(actual_terrain_toml, expected_terrain_toml);

        // assert scripts are created
        let script_path: PathBuf = script_path(central_dir.path(), &"none".to_string());
        assert!(
            fs::exists(&script_path)?,
            "expected terrain-none.zsh to be created in current directory"
        );

        let actual_script =
            fs::read_to_string(&script_path).expect("expected terrain-none.zsh to be readable");
        let expected_script = fs::read_to_string("./tests/data/terrain-none.empty.zsh")
            .expect("expected test script to be readable");

        assert_eq!(actual_script, expected_script);

        current_dir
            .close()
            .expect("expected directory to be cleaned up");

        restore_env_var(EDITOR.to_string(), editor);
        Ok(())
    }
}
