use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Ok, Result};
use home::home_dir;

use crate::types::errors::TerrainiumErrors;

pub fn get_config_path() -> Result<PathBuf> {
    let home = home_dir();
    if let Some(home) = home {
        if Path::is_dir(home.as_path()) {
            let config_dir = Path::join(&home, PathBuf::from(".config"));
            if Path::try_exists(&config_dir.as_path())? {
                return Ok(config_dir);
            }
            return Ok(home);
        } else {
            return Err(TerrainiumErrors::InvalidHomeDirectory.into());
        }
    }
    return Err(TerrainiumErrors::UnableToFindHome.into());
}

fn create_dir_if_not_exist(dir: &Path) -> Result<bool> {
    if !Path::try_exists(dir)? {
        println!("creating a directory at path {}", dir.to_string_lossy());
        std::fs::create_dir(dir)?;
        return Ok(true);
    }
    return Ok(false);
}

pub fn get_terrainium_config_path() -> Result<PathBuf> {
    return Ok(Path::join(&get_config_path()?, "terrainium"));
}

pub fn get_central_store_path() -> Result<PathBuf> {
    return Ok(Path::join(&get_terrainium_config_path()?, "terrains"));
}

pub fn create_config_dir() -> Result<PathBuf> {
    let config_path = get_terrainium_config_path()?;
    create_dir_if_not_exist(config_path.as_path())
        .context("unable to create terrainium config directory")?;
    create_dir_if_not_exist(get_central_store_path()?.as_path())
        .context("unable to create terrains directory in terrainium config directory")?;
    return Ok(config_path);
}

pub fn get_central_terrain_path() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let mut filename = Path::canonicalize(cwd.as_path())?
        .to_string_lossy()
        .to_string()
        .replace("/", "_");
    filename.push_str(".toml");
    return Ok(Path::join(
        &get_central_store_path()?,
        PathBuf::from(filename),
    ));
}

pub fn get_local_terrain_path() -> Result<PathBuf> {
    return Ok(Path::join(&std::env::current_dir()?, "terrain.toml"));
}

pub fn is_local_terrain_present() -> Result<bool> {
    return Ok(Path::try_exists(&get_local_terrain_path()?)?);
}

pub fn is_central_terrain_present() -> Result<bool> {
    return Ok(Path::try_exists(&get_central_terrain_path()?)?);
}

pub fn get_terrain_toml() -> Result<PathBuf> {
    if is_local_terrain_present()? {
        return get_local_terrain_path();
    } else if is_central_terrain_present()? {
        return get_central_terrain_path();
    } else {
        let err = anyhow!("unable to get terrain.toml for this project. initialize terrain with `terrainium init` command");
        return Err(err);
    }
}
