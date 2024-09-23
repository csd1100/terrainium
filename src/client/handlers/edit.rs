use crate::client::types::context::Context;
#[double]
use crate::common::execute::Run;
use crate::common::shell::Shell;
use crate::common::types::terrain::Terrain;
use anyhow::{Context as AnyhowContext, Result};
use mockall_double::double;
use std::fs;

const EDITOR: &str = "EDITOR";

pub fn handle(context: Context) -> Result<()> {
    let toml_path = context
        .toml_path()
        .context("failed to edit terrain because it does not exist.")?;

    let editor = std::env::var(EDITOR).unwrap_or_else(|_| {
        println!("the environment variable EDITOR not set. using 'vi' as text editor");
        "vi".to_string()
    });

    let edit = Run::new(
        editor,
        vec![toml_path
            .to_string_lossy()
            .parse()
            .context(format!("failed to convert path {:?} to string", toml_path))?],
        Some(std::env::vars().collect()),
    );

    edit.wait()
        .context(format!("failed to edit file {:?}", toml_path))?;

    let terrain = Terrain::from_toml(
        fs::read_to_string(&toml_path).context(format!("failed to read {:?}", toml_path))?,
    )
        .expect("terrain to be parsed from toml");

    context
        .shell()
        .generate_scripts(&context, terrain)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::client::handlers::init::test::{
        mock_runner_with_compile_expectations, script_path, scripts_dir,
        setup_mock_with_expectations,
    };
    use crate::client::types::context::Context;
    use crate::common::execute::test::{restore_env_var, set_env_var};
    use crate::common::execute::MockRun;
    use crate::common::shell::{Shell, Zsh};
    use anyhow::Result;
    use serial_test::serial;
    use std::fs;
    use std::os::unix::process::ExitStatusExt;
    use std::path::PathBuf;
    use std::process::ExitStatus;
    use tempfile::tempdir;

    const EDITOR: &str = "EDITOR";

    #[serial]
    #[test]
    fn edit_opens_editor_and_generates_scripts_current_dir() -> Result<()> {
        let editor = set_env_var(EDITOR.to_string(), "vim".to_string());

        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");

        let mut edit_run = MockRun::default();
        edit_run
            .expect_wait()
            .with()
            .times(1)
            .return_once(|| Ok(ExitStatus::from_raw(0)));
        let edit_mock = MockRun::new_context();
        edit_mock
            .expect()
            .withf(move |exe, args, envs| {
                exe == &"vim".to_string()
                    && *args == vec![terrain_toml.to_string_lossy()]
                    && envs.is_some()
            })
            .times(1)
            .return_once(|_, _, _| edit_run);

        // setup mock to assert scripts are compiled when init
        let central_dir_path: PathBuf = central_dir.path().into();
        let new_mock = MockRun::new_context();
        new_mock
            .expect()
            .withf(|exe, args, env| exe == "/bin/zsh" && args.is_empty() && env.is_none())
            .times(1)
            .returning(move |_, _, _| {
                let mock = MockRun::default();
                let mock = setup_mock_with_expectations(
                    mock,
                    mock_runner_with_compile_expectations(
                        central_dir_path.clone(),
                        "example_biome".to_string(),
                    ),
                );
                setup_mock_with_expectations(
                    mock,
                    mock_runner_with_compile_expectations(
                        central_dir_path.clone(),
                        "none".to_string(),
                    ),
                )
            });
        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::get(),
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        fs::copy("./tests/data/terrain.example.toml", terrain_toml)
            .expect("test file to be copied");

        let script_dir = scripts_dir(central_dir.path());
        fs::create_dir_all(script_dir).expect("test scripts dir to be created");

        super::handle(context).expect("no error to be thrown");

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

        restore_env_var(EDITOR.to_string(), editor);
        Ok(())
    }

    #[serial]
    #[test]
    fn edit_opens_editor_and_generates_scripts_central_dir() -> Result<()> {
        let editor = set_env_var(EDITOR.to_string(), "vim".to_string());

        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let mut terrain_toml: PathBuf = central_dir.path().into();
        terrain_toml.push("terrain.toml");

        let mut edit_run = MockRun::default();
        edit_run
            .expect_wait()
            .with()
            .times(1)
            .return_once(|| Ok(ExitStatus::from_raw(0)));
        let edit_mock = MockRun::new_context();
        edit_mock
            .expect()
            .withf(move |exe, args, envs| {
                exe == &"vim".to_string()
                    && *args == vec![terrain_toml.to_string_lossy()]
                    && envs.is_some()
            })
            .times(1)
            .return_once(|_, _, _| edit_run);

        // setup mock to assert scripts are compiled when init
        let central_dir_path: PathBuf = central_dir.path().into();
        let new_mock = MockRun::new_context();
        new_mock
            .expect()
            .withf(|exe, args, env| exe == "/bin/zsh" && args.is_empty() && env.is_none())
            .times(1)
            .returning(move |_, _, _| {
                let mock = MockRun::default();
                let mock = setup_mock_with_expectations(
                    mock,
                    mock_runner_with_compile_expectations(
                        central_dir_path.clone(),
                        "example_biome".to_string(),
                    ),
                );
                setup_mock_with_expectations(
                    mock,
                    mock_runner_with_compile_expectations(
                        central_dir_path.clone(),
                        "none".to_string(),
                    ),
                )
            });
        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::get(),
        );

        let mut terrain_toml: PathBuf = central_dir.path().into();
        terrain_toml.push("terrain.toml");
        fs::copy("./tests/data/terrain.example.toml", terrain_toml)
            .expect("test file to be copied");

        let script_dir = scripts_dir(central_dir.path());
        fs::create_dir_all(script_dir).expect("test scripts dir to be created");

        super::handle(context).expect("no error to be thrown");

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

        restore_env_var(EDITOR.to_string(), editor);
        Ok(())
    }

    #[serial]
    #[test]
    fn edit_returns_error_when_no_terrain() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let mock_run = MockRun::new_context();
        mock_run
            .expect()
            .withf(|_, _, _| true)
            .times(1)
            .return_once(|_, _, _| MockRun::default());

        let err = super::handle(Context::build_without_shell(
            current_dir.path().into(),
            central_dir.path().into(),
        ))
            .expect_err("expected to get error")
            .to_string();

        assert_eq!("failed to edit terrain because it does not exist.", err);

        Ok(())
    }

    #[serial]
    #[test]
    fn edit_opens_default_editor_if_env_not_set_and_generates_scripts() -> Result<()> {
        let editor = set_env_var(EDITOR.to_string(), "vim".to_string());
        std::env::remove_var(EDITOR);

        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");

        let mut edit_run = MockRun::default();
        edit_run
            .expect_wait()
            .with()
            .times(1)
            .return_once(|| Ok(ExitStatus::from_raw(0)));
        let edit_mock = MockRun::new_context();
        edit_mock
            .expect()
            .withf(move |exe, args, envs| {
                exe == &"vi".to_string()
                    && *args == vec![terrain_toml.to_string_lossy()]
                    && envs.is_some()
            })
            .times(1)
            .return_once(|_, _, _| edit_run);

        // setup mock to assert scripts are compiled when init
        let central_dir_path: PathBuf = central_dir.path().into();
        let new_mock = MockRun::new_context();
        new_mock
            .expect()
            .withf(|exe, args, env| exe == "/bin/zsh" && args.is_empty() && env.is_none())
            .times(1)
            .returning(move |_, _, _| {
                let mock = MockRun::default();
                let mock = setup_mock_with_expectations(
                    mock,
                    mock_runner_with_compile_expectations(
                        central_dir_path.clone(),
                        "example_biome".to_string(),
                    ),
                );
                setup_mock_with_expectations(
                    mock,
                    mock_runner_with_compile_expectations(
                        central_dir_path.clone(),
                        "none".to_string(),
                    ),
                )
            });
        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::get(),
        );

        let mut terrain_toml: PathBuf = current_dir.path().into();
        terrain_toml.push("terrain.toml");
        fs::copy("./tests/data/terrain.example.toml", terrain_toml)
            .expect("test file to be copied");

        let script_dir = scripts_dir(central_dir.path());
        fs::create_dir_all(script_dir).expect("test scripts dir to be created");

        super::handle(context).expect("no error to be thrown");

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

        restore_env_var(EDITOR.to_string(), editor);
        Ok(())
    }
}
