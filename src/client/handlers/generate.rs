use crate::client::shell::Shell;
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use anyhow::{Context as AnyhowContext, Result};
use std::fs::{create_dir_all, exists, read_to_string};

pub fn handle(context: Context) -> Result<()> {
    if !exists(context.scripts_dir()).context("failed to check if scripts dir exists")? {
        create_dir_all(context.scripts_dir()).context("failed to create scripts dir")?;
    }

    let terrain = Terrain::from_toml(read_to_string(context.toml_path()?).context(format!(
        "failed to read terrain.toml from path {:?}",
        context.toml_path()
    ))?)
    .expect("expected terrain to created from toml");

    context.shell().generate_scripts(&context, terrain)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::client::old_utils::test::{
        compile_expectations, script_path, scripts_dir, setup_command_runner_mock_with_expectations,
    };
    use crate::client::shell::Zsh;
    use crate::client::types::context::Context;
    use crate::common::execute::MockCommandToRun;
    use anyhow::Result;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    pub fn generates_script() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let central_dir_path: PathBuf = central_dir.path().into();
        let mock = MockCommandToRun::default();
        let mock = setup_command_runner_mock_with_expectations(
            mock,
            compile_expectations(central_dir_path.clone(), "example_biome".to_string()),
        );
        let mock = setup_command_runner_mock_with_expectations(
            mock,
            compile_expectations(central_dir_path.clone(), "none".to_string()),
        );
        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(mock),
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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

        Ok(())
    }

    #[test]
    pub fn creates_scripts_dir_if_necessary() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let central_dir_path: PathBuf = central_dir.path().into();
        let mock = MockCommandToRun::default();
        let mock = setup_command_runner_mock_with_expectations(
            mock,
            compile_expectations(central_dir_path.clone(), "example_biome".to_string()),
        );
        let mock = setup_command_runner_mock_with_expectations(
            mock,
            compile_expectations(central_dir_path.clone(), "none".to_string()),
        );
        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(mock),
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
        fs::copy("./tests/data/terrain.example.toml", terrain_toml)
            .expect("test file to be copied");

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

        Ok(())
    }
}
