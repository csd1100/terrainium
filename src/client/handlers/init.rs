use crate::client::types::context::Context;
use crate::common::constants::errors::TerrainiumErrors::AlreadyExists;
use crate::common::shell::Shell;
use crate::common::types::terrain::Terrain;
use anyhow::{anyhow, Context as AnyhowContext, Result};
use std::fs;
use std::fs::File;
use std::io::Write;

pub fn handle(context: Context, central: bool, example: bool) -> Result<()> {
    if fs::exists(context.toml_path(central)).context("failed to check if terrain.toml exists")? {
        return Err(anyhow!(AlreadyExists));
    }

    if !fs::exists(context.scripts_dir()).expect("failed to check if scripts dir exists") {
        fs::create_dir_all(context.scripts_dir()).context("failed to create scripts dir")?;
    }

    let mut file = File::create_new(context.toml_path(central))
        .context("error while creating terrain.toml")?;

    let terrain = if example {
        Terrain::example()
    } else {
        Terrain::default()
    };

    let toml_str = terrain.to_toml().expect("terrain to be parsed to toml");

    file.write(toml_str.as_ref())
        .context("failed to write terrain in toml file")?;

    context.shell().generate_scripts(&context, terrain)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::client::types::context::Context;
    use crate::common::execute::MockRun;
    use crate::common::shell::{Shell, Zsh};
    use anyhow::Result;
    use serial_test::serial;
    use std::fs;
    use std::os::unix::process::ExitStatusExt;
    use std::path::{Path, PathBuf};
    use std::process::{ExitStatus, Output};
    use tempfile::tempdir;

    #[serial]
    #[test]
    fn init_creates_terrain_toml_in_current_dir() -> Result<()> {
        // setup
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;
        // setup mock to assert scripts are compiled when init
        let central_dir_path: PathBuf = central_dir.path().into();
        let new_mock = MockRun::new_context();
        new_mock
            .expect()
            .withf(|exe, args, env| exe == "/bin/zsh" && args.is_empty() && env.is_none())
            .times(1)
            .returning(move |_, _, _| {
                let mock = MockRun::default();
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

        // execute
        super::handle(context, false, false)?;

        // assertions
        // assert terrain.toml is created
        let mut terrain_toml_path: PathBuf = current_dir.path().into();
        terrain_toml_path.push("terrain.toml");
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

    #[serial]
    #[test]
    fn init_creates_terrain_toml_in_central_dir() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        // setup mock to assert scripts are compiled when init
        let central_dir_path: PathBuf = central_dir.path().into();
        let new_mock = MockRun::new_context();
        new_mock
            .expect()
            .withf(|exe, args, env| exe == "/bin/zsh" && args.is_empty() && env.is_none())
            .times(1)
            .returning(move |_, _, _| {
                let mock = MockRun::default();
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

        // execute
        super::handle(context, true, false)?;

        // assertions
        // assert toml is created in central dir
        let mut terrain_toml_path: PathBuf = central_dir.path().into();
        terrain_toml_path.push("terrain.toml");

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

    #[serial]
    #[test]
    fn init_creates_scripts_dir_if_not_present() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        // setup mock to assert scripts are compiled when init
        let central_dir_path: PathBuf = central_dir.path().into();
        let new_mock = MockRun::new_context();
        new_mock
            .expect()
            .withf(|exe, args, env| exe == "/bin/zsh" && args.is_empty() && env.is_none())
            .times(1)
            .returning(move |_, _, _| {
                let mock = MockRun::default();
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

        super::handle(context, false, false)
            .expect("no error to be thrown when directory is not present");

        let mut scripts_dir_path: PathBuf = central_dir.path().into();
        scripts_dir_path.push("scripts");

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

    #[serial]
    #[test]
    fn init_creates_central_dir_if_not_present() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        // setup mock to assert scripts are compiled when init
        let central_dir_path: PathBuf = central_dir.path().into();
        let new_mock = MockRun::new_context();
        new_mock
            .expect()
            .withf(|exe, args, env| exe == "/bin/zsh" && args.is_empty() && env.is_none())
            .times(1)
            .returning(move |_, _, _| {
                let mock = MockRun::default();
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

        fs::remove_dir(&central_dir).expect("temp directory to be removed");

        super::handle(context, true, false)
            .expect("no error to be thrown when directory is not present");

        let mut terrain_toml_path: PathBuf = central_dir.path().into();
        terrain_toml_path.push("terrain.toml");

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

    #[serial]
    #[test]
    fn init_throws_error_if_terrain_toml_exists_in_current_dir() -> Result<()> {
        let current_dir = tempdir()?;

        let new_mock = MockRun::new_context();
        new_mock
            .expect()
            .withf(|_, _, _| true)
            .times(1)
            .returning(move |_, _, _| MockRun::default());

        let context: Context =
            Context::build(current_dir.path().into(), PathBuf::new(), Zsh::get());

        let mut terrain_toml_path: PathBuf = current_dir.path().into();
        terrain_toml_path.push("terrain.toml");

        fs::write(terrain_toml_path, "")?;

        let err = super::handle(context, false, false).expect_err("expected error to be thrown");

        assert_eq!(err.to_string(), "terrain for this project is already present. edit existing terrain with `terrain edit` command");

        current_dir
            .close()
            .expect("expected directory to be cleaned up");

        Ok(())
    }

    #[serial]
    #[test]
    fn init_throws_error_if_terrain_toml_exists_in_central_dir() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let new_mock = MockRun::new_context();
        new_mock
            .expect()
            .withf(|_, _, _| true)
            .times(1)
            .returning(move |_, _, _| MockRun::default());

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::get(),
        );

        let mut terrain_toml_path: PathBuf = central_dir.path().into();
        terrain_toml_path.push("terrain.toml");

        fs::write(terrain_toml_path, "")?;

        let err = super::handle(context, true, false).expect_err("expected error to be thrown");

        assert_eq!(err.to_string(), "terrain for this project is already present. edit existing terrain with `terrain edit` command");

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
    fn init_creates_example_terrain_toml_in_current_dir() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

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

        super::handle(context, false, true)?;

        // assert toml is created
        let mut terrain_toml_path: PathBuf = current_dir.path().into();
        terrain_toml_path.push("terrain.toml");

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
    fn init_creates_example_terrain_toml_in_central_dir() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

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

        super::handle(context, true, true)?;

        let mut terrain_toml_path: PathBuf = central_dir.path().into();
        terrain_toml_path.push("terrain.toml");

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

    fn setup_mock_with_expectations(mut mock: MockRun, mock_with_expectations: MockRun) -> MockRun {
        mock.expect_clone()
            .with()
            .times(1)
            .return_once(|| mock_with_expectations);
        mock
    }
    fn mock_runner_with_compile_expectations(central_dir: PathBuf, biome_name: String) -> MockRun {
        let compile_script_path = compiled_script_path(central_dir.as_path(), &biome_name);
        let script_path = script_path(central_dir.as_path(), &biome_name);

        let mut mock_runner = MockRun::default();
        let args = vec![
            "-c".to_string(),
            format!(
                "zcompile -URz {} {}",
                compile_script_path.to_string_lossy(),
                script_path.to_string_lossy()
            ),
        ];

        mock_runner
            .expect_set_args()
            .withf(move |actual_args| *actual_args == args)
            .returning(|_| ());

        mock_runner
            .expect_set_envs()
            .withf(move |envs| envs.is_none())
            .times(1)
            .returning(|_| ());

        mock_runner
            .expect_get_output()
            .with()
            .times(1)
            .returning(|| {
                Ok(Output {
                    status: ExitStatus::from_raw(0),
                    stdout: Vec::<u8>::from("/tmp/test/path"),
                    stderr: Vec::<u8>::new(),
                })
            });
        mock_runner
    }

    fn script_path(central_dir: &Path, biome_name: &String) -> PathBuf {
        let mut script_path = scripts_dir(central_dir).clone();
        script_path.set_file_name(format!("terrain-{}.zsh", biome_name));
        script_path
    }

    fn compiled_script_path(central_dir: &Path, biome_name: &String) -> PathBuf {
        let mut compile_script_path = scripts_dir(central_dir).clone();
        compile_script_path.set_file_name(format!("terrain-{}.zwc", biome_name));
        compile_script_path
    }

    fn scripts_dir(central_dir: &Path) -> PathBuf {
        let mut scripts_dir: PathBuf = central_dir.into();
        scripts_dir.push("scripts");
        scripts_dir
    }
}
