use crate::client::shell::Shell;
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use crate::common::execute::Execute;
#[mockall_double::double]
use crate::common::types::command::Command;
use anyhow::{Context as AnyhowContext, Result};
use std::path::Path;
use tracing::info;

const EDITOR: &str = "EDITOR";

pub fn handle(context: Context) -> Result<()> {
    run_editor(context.toml_path(), context.terrain_dir())?;

    let (terrain, _) = Terrain::get_validated_and_fixed_terrain(&context)?;
    context.shell().generate_scripts(&context, terrain)?;

    Ok(())
}

pub(crate) fn run_editor(toml_path: &Path, terrain_dir: &Path) -> Result<()> {
    let editor = std::env::var(EDITOR).unwrap_or_else(|_| {
        info!("the environment variable EDITOR not set. using 'vi' as text editor");
        "vi".to_string()
    });

    let edit = Command::new(
        editor,
        vec![toml_path
            .to_string_lossy()
            .parse()
            .context(format!("failed to convert path {:?} to string", toml_path))?],
        Some(std::env::vars().collect()),
        Some(terrain_dir.to_path_buf()),
    );

    edit.wait()
        .context(format!("failed to edit file {:?}", toml_path))?;

    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::client::shell::Zsh;
    use crate::client::test_utils::assertions::terrain::AssertTerrain;
    use crate::client::test_utils::assertions::zsh::ExpectZSH;
    use crate::client::test_utils::constants::{
        IN_CENTRAL_DIR, IN_CURRENT_DIR, WITH_EXAMPLE_TERRAIN_TOML,
    };
    use crate::client::test_utils::{restore_env_var, set_env_var};
    use crate::client::types::context::Context;
    use crate::common::constants::{EXAMPLE_BIOME, NONE};
    use crate::common::types::command::MockCommand;
    use anyhow::Result;
    use fs::{copy, create_dir_all};
    use serial_test::serial;
    use std::fs;
    use std::os::unix::process::ExitStatusExt;
    use std::path::PathBuf;
    use std::process::ExitStatus;
    use tempfile::tempdir;

    pub(crate) const EDITOR: &str = "EDITOR";

    #[serial]
    #[test]
    fn edit_opens_editor_and_generates_scripts_current_dir() -> Result<()> {
        let editor = set_env_var(EDITOR.to_string(), Some("vim".to_string()));

        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
        let terrain_dir = current_dir.path().to_path_buf();

        let mut edit_run = MockCommand::default();
        edit_run
            .expect_wait()
            .with()
            .times(1)
            .return_once(|| Ok(ExitStatus::from_raw(0)));
        let edit_mock = MockCommand::new_context();
        edit_mock
            .expect()
            .withf(move |exe, args, envs, cwd| {
                exe == &"vim".to_string()
                    && *args == vec![terrain_toml.to_string_lossy()]
                    && envs.is_some()
                    && cwd.clone().unwrap() == terrain_dir
            })
            .times(1)
            .return_once(|_, _, _, _| edit_run);

        // setup mock to assert scripts are compiled when edit
        let expected_shell_operation = ExpectZSH::to()
            .compile_terrain_script_for(EXAMPLE_BIOME, central_dir.path())
            .compile_terrain_script_for(NONE, central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(expected_shell_operation),
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
        copy("./tests/data/terrain.example.toml", terrain_toml).expect("test file to be copied");

        let script_dir = central_dir.path().join("scripts");
        create_dir_all(script_dir).expect("test scripts dir to be created");

        super::handle(context).expect("no error to be thrown");

        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
            .was_initialized(IN_CURRENT_DIR, WITH_EXAMPLE_TERRAIN_TOML)
            .script_was_created_for(NONE)
            .script_was_created_for(EXAMPLE_BIOME);

        restore_env_var(EDITOR.to_string(), editor);
        Ok(())
    }

    #[serial]
    #[test]
    fn edit_opens_editor_and_generates_scripts_central_dir() -> Result<()> {
        let editor = set_env_var(EDITOR.to_string(), Some("vim".to_string()));

        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let terrain_toml: PathBuf = central_dir.path().join("terrain.toml");
        let terrain_dir = current_dir.path().to_path_buf();

        let mut edit_run = MockCommand::default();
        edit_run
            .expect_wait()
            .with()
            .times(1)
            .return_once(|| Ok(ExitStatus::from_raw(0)));
        let edit_mock = MockCommand::new_context();
        edit_mock
            .expect()
            .withf(move |exe, args, envs, cwd| {
                exe == &"vim".to_string()
                    && *args == vec![terrain_toml.to_string_lossy()]
                    && envs.is_some()
                    && cwd.clone().unwrap() == terrain_dir
            })
            .times(1)
            .return_once(|_, _, _, _| edit_run);

        // setup mock to assert scripts are compiled when init
        let expected_shell_operation = ExpectZSH::to()
            .compile_terrain_script_for(EXAMPLE_BIOME, central_dir.path())
            .compile_terrain_script_for(NONE, central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            central_dir.path().join("terrain.toml"),
            Zsh::build(expected_shell_operation),
        );

        let terrain_toml: PathBuf = central_dir.path().join("terrain.toml");
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

        restore_env_var(EDITOR.to_string(), editor);
        Ok(())
    }

    #[serial]
    #[test]
    fn edit_opens_default_editor_if_env_not_set_and_generates_scripts() -> Result<()> {
        let editor = set_env_var(EDITOR.to_string(), Some("vim".to_string()));
        std::env::remove_var(EDITOR);

        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
        let terrain_dir = current_dir.path().to_path_buf();

        let mut edit_run = MockCommand::default();
        edit_run
            .expect_wait()
            .with()
            .times(1)
            .return_once(|| Ok(ExitStatus::from_raw(0)));
        let edit_mock = MockCommand::new_context();
        edit_mock
            .expect()
            .withf(move |exe, args, envs, cwd| {
                exe == &"vi".to_string()
                    && *args == vec![terrain_toml.to_string_lossy()]
                    && envs.is_some()
                    && cwd.clone().unwrap() == terrain_dir
            })
            .times(1)
            .return_once(|_, _, _, _| edit_run);

        // setup mock to assert scripts are compiled when init
        let expected_shell_operation = ExpectZSH::to()
            .compile_terrain_script_for(EXAMPLE_BIOME, central_dir.path())
            .compile_terrain_script_for(NONE, central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(expected_shell_operation),
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
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

        restore_env_var(EDITOR.to_string(), editor);
        Ok(())
    }
}
