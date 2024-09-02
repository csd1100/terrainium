use crate::types::errors::TerrainiumErrors;
use anyhow::Result;
use home::home_dir;
#[cfg(test)]
use mockall::automock;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct Paths {
    home_dir: PathBuf,
    current_dir: PathBuf,
}

impl Paths {
    pub fn get_cwd(&self) -> &PathBuf {
        &self.current_dir
    }

    pub fn get_home_dir(&self) -> &PathBuf {
        &self.home_dir
    }
}

pub fn get_paths(home_dir: PathBuf, current_dir: PathBuf) -> Result<Paths> {
    Ok(Paths {
        home_dir,
        current_dir,
    })
}

pub fn get_cwd() -> Result<PathBuf> {
    Ok(std::env::current_dir()?)
}

pub fn get_home_dir() -> Result<PathBuf> {
    let home = home_dir();
    if let Some(home) = home {
        if Path::is_dir(home.as_path()) {
            return Ok(home);
        }
    }
    Err(TerrainiumErrors::UnableToFindHome.into())
}

#[cfg_attr(test, automock)]
pub mod misc {
    use uuid::Uuid;

    pub fn get_uuid() -> String {
        Uuid::new_v4().to_string()
    }
}

pub mod test_helpers {
    use crate::helpers::utils::Paths;
    use anyhow::Result;
    use std::path::Path;

    pub fn generate_terrain_central_store_path(paths: &Paths) -> Result<String> {
        let terrain_dir = Path::canonicalize(paths.get_cwd().as_path())?
            .to_string_lossy()
            .to_string()
            .replace('/', "_");
        let central_store = Path::join(paths.get_home_dir(), ".config")
            .join("terrainium")
            .join("terrains")
            .join(terrain_dir);
        Ok(central_store.to_str().unwrap().to_string())
    }
    pub fn generate_terrain_compiled_zwc_path(paths: &Paths) -> Result<String> {
        let central_store = generate_terrain_central_store_path(paths)?;
        Ok(central_store + "/terrain-example_biome.zwc")
    }
}
