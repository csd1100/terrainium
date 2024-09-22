use crate::common::shell::{Shell, Zsh};
use home::home_dir;
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub struct Context {
    current_dir: PathBuf,
    central_dir: PathBuf,
    shell: Zsh,
}

const TERRAIN_TOML: &str = "terrain.toml";
const CONFIG_LOCATION: &str = ".config/terrainium";
const TERRAINS_DIR_NAME: &str = "terrains";
const SCRIPTS_DIR_NAME: &str = "scripts";

impl Default for Context {
    fn default() -> Self {
        Self::generate()
    }
}

impl Context {
    pub fn generate() -> Self {
        Context {
            current_dir: env::current_dir().expect("failed to get current directory"),
            central_dir: get_central_dir_location(
                env::current_dir().expect("failed to get current directory"),
            ),
            shell: Zsh::get(),
        }
    }
    pub fn current_dir(&self) -> &PathBuf {
        &self.current_dir
    }

    pub fn central_dir(&self) -> &PathBuf {
        &self.central_dir
    }

    pub fn toml_path(&self, central: bool) -> PathBuf {
        let mut base_dir: PathBuf = if central {
            self.central_dir.clone()
        } else {
            self.current_dir.clone()
        };
        base_dir.push(TERRAIN_TOML);
        base_dir
    }

    pub fn scripts_dir(&self) -> PathBuf {
        let mut scripts_dir = self.central_dir.clone();
        scripts_dir.push(SCRIPTS_DIR_NAME);
        scripts_dir
    }

    pub(crate) fn shell(&self) -> &Zsh {
        &self.shell
    }

    #[cfg(test)]
    pub(crate) fn build(current_dir: PathBuf, central_dir: PathBuf, shell: Zsh) -> Self {
        Context {
            current_dir,
            central_dir,
            shell,
        }
    }
}

fn get_central_dir_location(current_dir: PathBuf) -> PathBuf {
    let mut central_dir = home_dir().expect("failed to get home directory");
    central_dir.push(CONFIG_LOCATION);
    central_dir.push(TERRAINS_DIR_NAME);

    let terrain_dir_name = Path::canonicalize(current_dir.as_path())
        .expect("expected current directory to be valid")
        .to_string_lossy()
        .to_string()
        .replace('/', "_");
    central_dir.push(terrain_dir_name);

    central_dir
}

#[cfg(test)]
mod test {
    use super::Context;
    use crate::common::execute::MockRun;
    use crate::common::shell::{Shell, Zsh};
    use anyhow::Result;
    use home::home_dir;
    use serial_test::serial;
    use std::env;
    use std::path::{Path, PathBuf};

    #[serial]
    #[test]
    fn new_creates_context() -> Result<()> {
        let current_dir = env::current_dir().expect("failed to get current directory");
        let central_dir = get_central_dir_location();

        let new_mock = MockRun::new_context();
        new_mock
            .expect()
            .withf(|_, _, _| true)
            .times(1)
            .returning(|_, _, _| MockRun::default());

        let actual = Context::generate();
        assert_eq!(current_dir, actual.current_dir);
        assert_eq!(central_dir, actual.central_dir);

        Ok(())
    }

    #[serial]
    #[test]
    fn current_dir_returns_current_dir() -> Result<()> {
        let new_mock = MockRun::new_context();
        new_mock
            .expect()
            .withf(|_, _, _| true)
            .times(1)
            .returning(|_, _, _| MockRun::default());

        let context = Context::generate();
        assert_eq!(
            &env::current_dir().expect("failed to get current directory"),
            context.current_dir()
        );
        Ok(())
    }

    #[serial]
    #[test]
    fn toml_path_return_current_terrain_toml_path() -> Result<()> {
        let new_mock = MockRun::new_context();
        new_mock
            .expect()
            .withf(|_, _, _| true)
            .times(1)
            .returning(|_, _, _| MockRun::default());

        let mut expected_terrain_toml =
            env::current_dir().expect("failed to get current directory");
        expected_terrain_toml.push("terrain.toml");

        let context = Context::generate();

        assert_eq!(expected_terrain_toml, context.toml_path(false));

        Ok(())
    }

    #[serial]
    #[test]
    fn toml_path_return_central_terrain_toml_path() -> Result<()> {
        let new_mock = MockRun::new_context();
        new_mock
            .expect()
            .withf(|_, _, _| true)
            .times(1)
            .returning(|_, _, _| MockRun::default());

        let mut expected_terrain_toml = get_central_dir_location();
        expected_terrain_toml.push("terrain.toml");

        let context = Context::generate();

        assert_eq!(expected_terrain_toml, context.toml_path(true));

        Ok(())
    }

    #[serial]
    #[test]
    fn central_dir_returns_config_location() -> Result<()> {
        let new_mock = MockRun::new_context();
        new_mock
            .expect()
            .withf(|_, _, _| true)
            .times(1)
            .returning(|_, _, _| MockRun::default());

        let context = Context::generate();
        let central_dir = get_central_dir_location();

        assert_eq!(&central_dir, context.central_dir());
        Ok(())
    }

    #[serial]
    #[test]
    fn scripts_dir_returns_config_location() -> Result<()> {
        let new_mock = MockRun::new_context();
        new_mock
            .expect()
            .withf(|_, _, _| true)
            .times(1)
            .returning(|_, _, _| MockRun::default());

        let context = Context::generate();
        let mut scripts_dir = get_central_dir_location();
        scripts_dir.push("scripts");

        assert_eq!(scripts_dir, context.scripts_dir());
        Ok(())
    }

    #[serial]
    #[test]
    fn shell_returns_shell() -> Result<()> {
        let new_mock = MockRun::new_context();
        new_mock
            .expect()
            .withf(|_, _, _| true)
            .times(1)
            .returning(move |_, _, _| MockRun::default());

        let context = Context::generate();

        let expected_shell = Zsh::build(MockRun::default());

        assert_eq!(expected_shell.exe(), context.shell().exe());

        Ok(())
    }

    fn get_central_dir_location() -> PathBuf {
        let current_dir = env::current_dir().expect("failed to get current directory");
        let terrain_dir_name = Path::canonicalize(current_dir.as_path())
            .expect("current directory to be present")
            .to_string_lossy()
            .to_string()
            .replace('/', "_");

        let mut central_dir = home_dir().expect("failed to get home directory");
        central_dir.push(".config/terrainium/terrains");
        central_dir.push(terrain_dir_name);

        central_dir
    }
}
