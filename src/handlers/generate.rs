use anyhow::{anyhow, Result};

use crate::helpers::utils::Paths;
use crate::shell::zsh::ops::generate_and_compile;
use crate::{
    helpers::operations::{get_central_store_path, get_current_dir_toml, remove_all_script_files},
    types::terrain::{parse_terrain_from, Terrain},
};

pub fn handle(paths: &Paths) -> Result<()> {
    let terrain = parse_terrain_from(get_current_dir_toml(paths)?)?;
    generate_and_compile_all(terrain, paths)
}

pub fn generate_and_compile_all(terrain: Terrain, paths: &Paths) -> Result<()> {
    let central_store = get_central_store_path(paths)?;

    remove_all_script_files(central_store.as_path())?;

    let result: Result<Vec<_>> = terrain
        .into_iter()
        .map(|(biome_name, environment)| {
            generate_and_compile(&central_store, biome_name, environment)
        })
        .collect();

    if result.is_err() {
        Err(anyhow!(format!(
            "Error while generating and compiling scripts, error: {}",
            result.unwrap_err()
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::helpers::utils::get_paths;
    use crate::helpers::utils::test_helpers::generate_terrain_central_store_path;
    use crate::shell::process::mock_spawn;
    use anyhow::Result;
    use clap::builder::OsStr;
    use serial_test::serial;
    use std::fs;
    use std::os::unix::prelude::ExitStatusExt;
    use std::path::PathBuf;
    use std::process::{ExitStatus, Output};
    use tempfile::tempdir;

    #[test]
    #[serial]
    fn handle_generate_generates_scripts() -> Result<()> {
        // setup terrain.toml
        let test_dir = tempdir()?;
        let home_dir = tempdir()?;
        let test_dir_path: PathBuf = test_dir.path().into();
        let home_dir_path: PathBuf = home_dir.path().into();

        let paths = get_paths(home_dir_path, test_dir_path)?;

        let mut terrain_toml_path: PathBuf = test_dir.path().into();
        terrain_toml_path.push("terrain.toml");
        fs::copy("./tests/data/terrain.full.toml", &terrain_toml_path)?;

        let central_storage = generate_terrain_central_store_path(&paths)?;
        let mock_spawn_get_output = mock_spawn::and_get_output_context();

        let exp_example_biome_args = vec![
            "-c".to_owned(),
            "zcompile -URz ".to_owned()
                + &central_storage
                + "/terrain-example_biome.zwc "
                + &central_storage
                + "/terrain-example_biome.zsh",
        ];

        mock_spawn_get_output
            .expect()
            .withf(move |exe, args, envs| {
                let exe_eq = exe == "/bin/zsh";
                let args_eq = *args == exp_example_biome_args;
                let envs_eq = envs.is_none();
                exe_eq && args_eq && envs_eq
            })
            .return_once(|_, _, _| {
                Ok(Output {
                    status: ExitStatus::from_raw(0),
                    stdout: Vec::<u8>::new(),
                    stderr: Vec::<u8>::new(),
                })
            });

        let exp_example_biome2_args = vec![
            "-c".to_owned(),
            "zcompile -URz ".to_owned()
                + &central_storage
                + "/terrain-example_biome2.zwc "
                + &central_storage
                + "/terrain-example_biome2.zsh",
        ];

        mock_spawn_get_output
            .expect()
            .withf(move |exe, args, envs| {
                let exe_eq = exe == "/bin/zsh";
                let args_eq = *args == exp_example_biome2_args;
                let envs_eq = envs.is_none();
                exe_eq && args_eq && envs_eq
            })
            .return_once(|_, _, _| {
                Ok(Output {
                    status: ExitStatus::from_raw(0),
                    stdout: Vec::<u8>::new(),
                    stderr: Vec::<u8>::new(),
                })
            });

        let exp_none_args = vec![
            "-c".to_owned(),
            "zcompile -URz ".to_owned()
                + &central_storage
                + "/terrain-none.zwc "
                + &central_storage
                + "/terrain-none.zsh",
        ];

        mock_spawn_get_output
            .expect()
            .withf(move |exe, args, envs| {
                let exe_eq = exe == "/bin/zsh";
                let args_eq = *args == exp_none_args;
                let envs_eq = envs.is_none();
                exe_eq && args_eq && envs_eq
            })
            .return_once(|_, _, _| {
                Ok(Output {
                    status: ExitStatus::from_raw(0),
                    stdout: Vec::<u8>::new(),
                    stderr: Vec::<u8>::new(),
                })
            });

        fs::create_dir_all(PathBuf::from(&central_storage)).expect("to be created");
        super::handle(&paths)?;

        let mut assertion_counter = 0;
        for entry in fs::read_dir(PathBuf::from(&central_storage))? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().expect("to have extension") == OsStr::from("zsh") {
                let expected = fs::read_to_string(
                    "./tests/data/".to_owned() + entry.file_name().to_str().expect("to exist"),
                )
                .expect("to be present");
                let actual = fs::read_to_string(path).expect("to be present");
                assert_eq!(
                    expected,
                    actual,
                    "failed to assert values for file {:?}",
                    entry.file_name()
                );
                assertion_counter += 1;
            }
        }
        assert_eq!(3, assertion_counter, "expected 3 zsh file assertions");

        Ok(())
    }
}
