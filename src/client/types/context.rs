use std::env;
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub struct Context {
    current_dir: PathBuf,
}

const TERRAIN_TOML: &'static str = "terrain.toml";

impl Context {
    pub fn new() -> Self {
        Context {
            current_dir: env::current_dir().expect("failed to get current directory")
        }
    }
    pub fn current_dir(&self) -> &PathBuf {
        &self.current_dir
    }

    pub fn get_toml_path(&self) -> PathBuf {
        let mut terrain_toml = self.current_dir.clone();
        terrain_toml.push(TERRAIN_TOML);
        terrain_toml
    }

    #[cfg(test)]
    pub(crate) fn build(current_dir: PathBuf) -> Self {
        Context {
            current_dir
        }
    }
}

#[cfg(test)]
mod test {
    use super::Context;
    use anyhow::Result;
    use std::env;

    #[test]
    fn new_creates_context() -> Result<()> {
        let expected = Context {
            current_dir: env::current_dir().expect("failed to get current directory")
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
    fn toml_path_return_terrain_toml_path() -> Result<()> {
        let mut expected_terrain_toml = env::current_dir().expect("failed to get current directory");
        expected_terrain_toml.push("terrain.toml");

        let context = Context::new();

        assert_eq!(expected_terrain_toml, context.get_toml_path());

        Ok(())
    }
}