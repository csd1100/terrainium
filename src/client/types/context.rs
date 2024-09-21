use home::home_dir;
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub struct Context {
    current_dir: PathBuf,
    central_dir: PathBuf,
}

const TERRAIN_TOML: &'static str = "terrain.toml";
const CONFIG_LOCATION: &'static str = ".config/terrainium";
const TERRAINS_DIR_NAME: &'static str = "terrains";

impl Context {
    pub fn new() -> Self {
        Context {
            current_dir: env::current_dir().expect("failed to get current directory"),
            central_dir: get_central_dir_location(env::current_dir().expect("failed to get current directory")),
        }
    }
    pub fn current_dir(&self) -> &PathBuf {
        &self.current_dir
    }

    pub fn central_dir(&self) -> &PathBuf {
        &self.central_dir
    }

    pub fn get_toml_path(&self, central: bool) -> PathBuf {
        let mut base_dir: PathBuf = if central {
            self.central_dir.clone()
        } else {
            self.current_dir.clone()
        };
        base_dir.push(TERRAIN_TOML);
        base_dir
    }

    #[cfg(test)]
    pub(crate) fn build(current_dir: PathBuf, central_dir: PathBuf) -> Self {
        Context {
            current_dir,
            central_dir,
        }
    }
}

fn get_central_dir_location(current_dir: PathBuf) -> PathBuf {
    let mut central_dir = home_dir().expect("failed to get home directory");
    central_dir.push(CONFIG_LOCATION);
    central_dir.push(TERRAINS_DIR_NAME);

    let terrain_dir_name = Path::canonicalize(current_dir.as_path()).expect("expected current directory to be valid")
        .to_string_lossy()
        .to_string()
        .replace('/', "_");
    central_dir.push(terrain_dir_name);

    central_dir
}

#[cfg(test)]
mod test {
    use super::Context;
    use anyhow::Result;
    use home::home_dir;
    use std::env;
    use std::path::{Path, PathBuf};

    #[test]
    fn new_creates_context() -> Result<()> {
        let expected = Context {
            current_dir: env::current_dir().expect("failed to get current directory"),
            central_dir: get_central_dir_location(),
        };
        assert_eq!(expected, Context::new());
        Ok(())
    }

    #[test]
    fn current_dir_returns_current_dir() -> Result<()> {
        let context = Context::new();
        assert_eq!(&env::current_dir().expect("failed to get current directory"), context.current_dir());
        Ok(())
    }

    #[test]
    fn toml_path_return_current_terrain_toml_path() -> Result<()> {
        let mut expected_terrain_toml = env::current_dir().expect("failed to get current directory");
        expected_terrain_toml.push("terrain.toml");

        let context = Context::new();

        assert_eq!(expected_terrain_toml, context.get_toml_path(false));

        Ok(())
    }

    #[test]
    fn toml_path_return_central_terrain_toml_path() -> Result<()> {
        let mut expected_terrain_toml = get_central_dir_location();
        expected_terrain_toml.push("terrain.toml");

        let context = Context::new();

        assert_eq!(expected_terrain_toml, context.get_toml_path(true));

        Ok(())
    }
    #[test]
    fn central_dir_returns_config_location() -> Result<()> {
        let context = Context::new();
        let central_dir = get_central_dir_location();

        assert_eq!(&central_dir, context.central_dir());
        Ok(())
    }

    fn get_central_dir_location() -> PathBuf {
        let current_dir = env::current_dir().expect("failed to get current directory");
        let terrain_dir_name = Path::canonicalize(current_dir.as_path()).expect("current directory to be present")
            .to_string_lossy()
            .to_string()
            .replace('/', "_");

        let mut central_dir = home_dir().expect("failed to get home directory");
        central_dir.push(".config/terrainium/terrains");
        central_dir.push(terrain_dir_name);

        central_dir
    }
}