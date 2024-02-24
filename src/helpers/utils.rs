#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub mod fs {
    use crate::types::errors::TerrainiumErrors;
    use anyhow::Result;
    use home::home_dir;
    use std::path::{Path, PathBuf};

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
}

#[cfg_attr(test, automock)]
pub mod misc {
    use uuid::Uuid;

    pub fn get_uuid() -> String {
        Uuid::new_v4().to_string()
    }
}
