use crate::client::types::context::Context;
use crate::common::types::terrain::Terrain;
use anyhow::{anyhow, Context as AnyhowContext, Result};
use std::fs;
use std::fs::File;
use std::io::Write;

pub fn handle(context: Context, central: bool, example: bool) -> Result<()> {
    if fs::exists(context.get_toml_path(central)).expect("failed to check if terrain.toml exists") {
        return Err(anyhow!("terrain for this project is already present. edit existing terrain with `terrain edit` command"));
    }

    if central
        && !fs::exists(context.central_dir()).expect("failed to check if central directory exists")
    {
        fs::create_dir_all(context.central_dir()).expect("failed to create central directory");
    }

    let mut file = File::create_new(context.get_toml_path(central))
        .context("error while creating terrain.toml")?;

    let toml_str = if example {
        Terrain::example()
            .to_toml()
            .expect("expected example terrain to be converted to toml")
    } else {
        Terrain::default()
            .to_toml()
            .expect("expected default terrain to be converted to toml")
    };

    file.write(toml_str.as_ref())
        .context("failed to write terrain in toml file")?;

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::client::types::context::Context;
    use anyhow::Result;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn init_creates_terrain_toml_in_current_dir() -> Result<()> {
        let current_dir = tempdir()?;

        let context: Context = Context::build(current_dir.path().into(), PathBuf::new());

        super::handle(context, false, false)?;

        let mut terrain_toml_path: PathBuf = current_dir.path().into();
        terrain_toml_path.push("terrain.toml");

        assert!(
            fs::exists(&terrain_toml_path)?,
            "expected terrain.toml to be created in current directory"
        );

        let actual =
            fs::read_to_string(&terrain_toml_path).expect("expected terrain.toml to be readable");
        let expected = fs::read_to_string("./tests/data/terrain.empty.toml")
            .expect("expected test toml to be readable");

        assert_eq!(actual, expected);

        current_dir
            .close()
            .expect("expected directory to be cleaned up");

        Ok(())
    }

    #[test]
    fn init_creates_terrain_toml_in_central_dir() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let context: Context = Context::build(current_dir.path().into(), central_dir.path().into());

        super::handle(context, true, false)?;

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

    #[test]
    fn init_creates_central_dir_if_not_present() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let context: Context = Context::build(current_dir.path().into(), central_dir.path().into());

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

    #[test]
    fn init_throws_error_if_terrain_toml_exists_in_current_dir() -> Result<()> {
        let current_dir = tempdir()?;

        let context: Context = Context::build(current_dir.path().into(), PathBuf::new());

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

    #[test]
    fn init_throws_error_if_terrain_toml_exists_in_central_dir() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let context: Context = Context::build(current_dir.path().into(), central_dir.path().into());

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

    #[test]
    fn init_creates_example_terrain_toml_in_current_dir() -> Result<()> {
        let current_dir = tempdir()?;

        let context: Context = Context::build(current_dir.path().into(), PathBuf::new());

        super::handle(context, false, true)?;

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

        current_dir
            .close()
            .expect("expected directory to be cleaned up");

        Ok(())
    }

    #[test]
    fn init_creates_example_terrain_toml_in_central_dir() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let context: Context = Context::build(current_dir.path().into(), central_dir.path().into());

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

        current_dir
            .close()
            .expect("expected directory to be cleaned up");
        central_dir
            .close()
            .expect("expected directory to be cleaned up");

        Ok(())
    }
}
