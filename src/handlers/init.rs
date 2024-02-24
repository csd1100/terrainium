use anyhow::{anyhow, Context, Result};
use mockall_double::double;
use std::path::PathBuf;

use crate::{
    helpers::operations::{
        create_config_dir, get_central_terrain_path, get_local_terrain_path, is_terrain_present,
        write_terrain,
    },
    types::terrain::Terrain,
};

#[double]
use crate::shell::editor::edit;

use super::generate::generate_and_compile_all;

pub fn handle(central: bool, example: bool, edit: bool) -> Result<()> {
    if is_terrain_present().context("failed to validate if terrain already exists")? {
        return Err(anyhow!(
            "terrain for this project is already present. edit existing terrain with `terrain edit` command"
        ));
    }

    create_config_dir().context("unable to create config directory")?;

    let terrain_toml_path: PathBuf = if central {
        get_central_terrain_path().context("unable to get central toml path")?
    } else {
        get_local_terrain_path().context("unable to get local terrain.toml")?
    };

    let terrain: Terrain = if example {
        Terrain::example()
    } else {
        Terrain::new()
    };

    write_terrain(&terrain_toml_path, &terrain)
        .context("failed to write generated terrain to toml file")?;

    println!(
        "terrain created at path {}",
        terrain_toml_path.to_string_lossy()
    );

    generate_and_compile_all(terrain)?;

    if edit {
        println!("editing...");
        edit::file(&terrain_toml_path).context("failed to edit terrain.toml")?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use std::path::{Path, PathBuf};

    use anyhow::Result;
    use mockall::predicate::eq;
    use serial_test::serial;
    use tempfile::tempdir;

    use crate::{
        helpers::utils::mock_fs,
        shell::{editor::mock_edit, zsh::mock_ops},
        types::{args::BiomeArg, terrain::Terrain},
    };

    #[test]
    #[serial]
    fn init_without_any_options_creates_and_compiles_terrain() -> Result<()> {
        let test_dir = tempdir()?;
        let home_dir = tempdir()?;

        let test_dir_path: PathBuf = test_dir.path().into();
        let mock_cwd = mock_fs::get_cwd_context();
        mock_cwd
            .expect()
            .returning(move || {
                let test_dir_path: PathBuf = test_dir_path.clone();
                Ok(test_dir_path)
            })
            .times(5);

        let home_dir_path: PathBuf = home_dir.path().into();
        let mock_home = mock_fs::get_home_dir_context();
        mock_home
            .expect()
            .returning(move || {
                let home_dir_path: PathBuf = home_dir_path.clone();
                Ok(home_dir_path)
            })
            .times(3);

        let home_dir_path: PathBuf = home_dir.path().into();
        let test_dir_path: PathBuf = test_dir.path().into();
        let scripts_dir_name = Path::canonicalize(test_dir_path.as_path())?
            .to_string_lossy()
            .to_string()
            .replace('/', "_");
        let scripts_dir_path = home_dir_path.join(PathBuf::from(
            ".config/terrainium/terrains/".to_owned() + &scripts_dir_name,
        ));

        let terrain = Terrain::new().get(Some(BiomeArg::None))?;
        let generate_and_compile_context = mock_ops::generate_and_compile_context();
        generate_and_compile_context
            .expect()
            .with(
                eq(scripts_dir_path.clone()),
                eq(String::from("none")),
                eq(terrain),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        super::handle(false, false, false)?;

        let mut actual_file_path = test_dir_path.clone();
        actual_file_path.push("terrain.toml");

        let expected =
            std::fs::read_to_string("./example_configs/terrain.empty.toml").expect("to be present");
        let actual = std::fs::read_to_string(actual_file_path).expect("to be present");
        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    #[serial]
    fn init_with_example_creates_and_compiles_terrain() -> Result<()> {
        let test_dir = tempdir()?;
        let home_dir = tempdir()?;

        let test_dir_path: PathBuf = test_dir.path().into();
        let mock_cwd = mock_fs::get_cwd_context();
        mock_cwd
            .expect()
            .returning(move || {
                let test_dir_path: PathBuf = test_dir_path.clone();
                Ok(test_dir_path)
            })
            .times(5);

        let home_dir_path: PathBuf = home_dir.path().into();
        let mock_home = mock_fs::get_home_dir_context();
        mock_home
            .expect()
            .returning(move || {
                let home_dir_path: PathBuf = home_dir_path.clone();
                Ok(home_dir_path)
            })
            .times(3);

        let home_dir_path: PathBuf = home_dir.path().into();
        let test_dir_path: PathBuf = test_dir.path().into();
        let scripts_dir_name = Path::canonicalize(test_dir_path.as_path())?
            .to_string_lossy()
            .to_string()
            .replace('/', "_");
        let scripts_dir_path = home_dir_path.join(PathBuf::from(
            ".config/terrainium/terrains/".to_owned() + &scripts_dir_name,
        ));

        let terrain = Terrain::example();
        let generate_and_compile_context = mock_ops::generate_and_compile_context();

        let main = terrain.get(Some(BiomeArg::None))?;
        generate_and_compile_context
            .expect()
            .with(
                eq(scripts_dir_path.clone()),
                eq(String::from("none")),
                eq(main),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        let example_biome = terrain.get(Some(BiomeArg::Value("example_biome".to_owned())))?;
        generate_and_compile_context
            .expect()
            .with(
                eq(scripts_dir_path.clone()),
                eq(String::from("example_biome")),
                eq(example_biome),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        super::handle(false, true, false)?;

        let mut actual_file_path = test_dir_path.clone();
        actual_file_path.push("terrain.toml");

        let expected = std::fs::read_to_string("./example_configs/terrain.default.toml")
            .expect("to be present");
        let actual = std::fs::read_to_string(actual_file_path).expect("to be present");
        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    #[serial]
    fn init_with_central_creates_and_compiles_terrain() -> Result<()> {
        let test_dir = tempdir()?;
        let home_dir = tempdir()?;

        let test_dir_path: PathBuf = test_dir.path().into();
        let mock_cwd = mock_fs::get_cwd_context();
        mock_cwd
            .expect()
            .returning(move || {
                let test_dir_path: PathBuf = test_dir_path.clone();
                Ok(test_dir_path)
            })
            .times(5);

        let home_dir_path: PathBuf = home_dir.path().into();
        let mock_home = mock_fs::get_home_dir_context();
        mock_home
            .expect()
            .returning(move || {
                let home_dir_path: PathBuf = home_dir_path.clone();
                Ok(home_dir_path)
            })
            .times(4);

        let home_dir_path: PathBuf = home_dir.path().into();
        let test_dir_path: PathBuf = test_dir.path().into();
        let scripts_dir_name = Path::canonicalize(test_dir_path.as_path())?
            .to_string_lossy()
            .to_string()
            .replace('/', "_");
        let scripts_dir_path = home_dir_path.join(PathBuf::from(
            ".config/terrainium/terrains/".to_owned() + &scripts_dir_name,
        ));

        let terrain = Terrain::new().get(Some(BiomeArg::None))?;
        let generate_and_compile_context = mock_ops::generate_and_compile_context();
        generate_and_compile_context
            .expect()
            .with(
                eq(scripts_dir_path.clone()),
                eq(String::from("none")),
                eq(terrain),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        super::handle(true, false, false)?;

        let mut actual_file_path = scripts_dir_path.clone();
        actual_file_path.push("terrain.toml");

        let expected =
            std::fs::read_to_string("./example_configs/terrain.empty.toml").expect("to be present");
        let actual = std::fs::read_to_string(actual_file_path).expect("to be present");
        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    #[serial]
    fn init_with_edit_creates_and_compiles_terrain_and_starts_editor() -> Result<()> {
        let test_dir = tempdir()?;
        let home_dir = tempdir()?;

        let test_dir_path: PathBuf = test_dir.path().into();
        let mock_cwd = mock_fs::get_cwd_context();
        mock_cwd
            .expect()
            .returning(move || {
                let test_dir_path: PathBuf = test_dir_path.clone();
                Ok(test_dir_path)
            })
            .times(5);

        let home_dir_path: PathBuf = home_dir.path().into();
        let mock_home = mock_fs::get_home_dir_context();
        mock_home
            .expect()
            .returning(move || {
                let home_dir_path: PathBuf = home_dir_path.clone();
                Ok(home_dir_path)
            })
            .times(3);

        let home_dir_path: PathBuf = home_dir.path().into();
        let test_dir_path: PathBuf = test_dir.path().into();
        let scripts_dir_name = Path::canonicalize(test_dir_path.as_path())?
            .to_string_lossy()
            .to_string()
            .replace('/', "_");
        let scripts_dir_path = home_dir_path.join(PathBuf::from(
            ".config/terrainium/terrains/".to_owned() + &scripts_dir_name,
        ));

        let terrain = Terrain::new().get(Some(BiomeArg::None))?;
        let generate_and_compile_context = mock_ops::generate_and_compile_context();
        generate_and_compile_context
            .expect()
            .with(
                eq(scripts_dir_path.clone()),
                eq(String::from("none")),
                eq(terrain),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        let mut actual_file_path = test_dir_path.clone();
        actual_file_path.push("terrain.toml");
        let mock_edit_file = mock_edit::file_context();
        mock_edit_file
            .expect()
            .with(eq(actual_file_path))
            .return_once(|_| Ok(()))
            .times(1);

        super::handle(false, false, true)?;

        let mut actual_file_path = test_dir_path.clone();
        actual_file_path.push("terrain.toml");

        let expected =
            std::fs::read_to_string("./example_configs/terrain.empty.toml").expect("to be present");
        let actual = std::fs::read_to_string(actual_file_path).expect("to be present");
        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    #[serial]
    fn init_with_local_terrain_present_throws_error() -> Result<()> {
        let test_dir = tempdir()?;

        let test_dir_path: PathBuf = test_dir.path().into();
        let mock_cwd = mock_fs::get_cwd_context();
        mock_cwd
            .expect()
            .returning(move || {
                let test_dir_path: PathBuf = test_dir_path.clone();
                Ok(test_dir_path)
            })
            .times(1);

        let mut terrain_toml_path = test_dir.into_path();
        terrain_toml_path.push("terrain.toml");

        // create a empty terrain toml file
        std::fs::write(terrain_toml_path, "")?;

        let err = super::handle(false, false, false).unwrap_err().to_string();

        assert_eq!(err, "terrain for this project is already present. edit existing terrain with `terrain edit` command");

        Ok(())
    }

    #[test]
    #[serial]
    fn init_with_central_terrain_present_throws_error() -> Result<()> {
        let test_dir = tempdir()?;
        let home_dir = tempdir()?;

        let test_dir_path: PathBuf = test_dir.path().into();
        let mock_cwd = mock_fs::get_cwd_context();
        mock_cwd
            .expect()
            .returning(move || {
                let test_dir_path: PathBuf = test_dir_path.clone();
                Ok(test_dir_path)
            })
            .times(2);

        let home_dir_path: PathBuf = home_dir.path().into();
        let mock_home = mock_fs::get_home_dir_context();
        mock_home
            .expect()
            .returning(move || {
                let home_dir_path: PathBuf = home_dir_path.clone();
                Ok(home_dir_path)
            })
            .times(1);

        let home_dir_path: PathBuf = home_dir.path().into();
        let test_dir_path: PathBuf = test_dir.path().into();
        let scripts_dir_name = Path::canonicalize(test_dir_path.as_path())?
            .to_string_lossy()
            .to_string()
            .replace('/', "_");
        let scripts_dir_path = home_dir_path.join(PathBuf::from(
            ".config/terrainium/terrains/".to_owned() + &scripts_dir_name,
        ));
        std::fs::create_dir_all(&scripts_dir_path)?;

        let mut terrain_toml_path = scripts_dir_path.clone();
        terrain_toml_path.push("terrain.toml");
        // create a empty terrain toml file
        std::fs::write(terrain_toml_path, "")?;

        let err = super::handle(false, false, false).unwrap_err().to_string();

        assert_eq!(err, "terrain for this project is already present. edit existing terrain with `terrain edit` command");

        Ok(())
    }
}
