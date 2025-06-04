use crate::client::shell::{Shell, Zsh};
use crate::client::types::config::Config;
use crate::common::constants::{
    CONFIG_LOCATION, SHELL_INTEGRATION_SCRIPTS_DIR, TERRAIN_SESSION_ID,
};
use anyhow::{bail, Context as AnyhowContext, Result};
use home::home_dir;
use std::env;
use std::env::current_dir;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Context {
    session_id: Option<String>,
    terrain_dir: PathBuf,
    central_dir: PathBuf,
    toml_path: PathBuf,
    config: Config,
    shell: Zsh,
}

const TERRAIN_TOML: &str = "terrain.toml";
const TERRAINS_DIR_NAME: &str = "terrains";
const SCRIPTS_DIR_NAME: &str = "scripts";

fn is_terrain_present(cwd: &Path) -> Option<PathBuf> {
    let local_toml = cwd.join(TERRAIN_TOML);
    let central_toml = get_central_dir_location(cwd).join(TERRAIN_TOML);

    if local_toml.exists() {
        return Some(local_toml);
    } else if central_toml.exists() {
        return Some(central_toml);
    }

    None
}

fn get_central_dir_location(terrain_dir: &Path) -> PathBuf {
    let terrain_dir_name = Path::canonicalize(terrain_dir)
        .expect("expected current directory to be valid")
        .to_string_lossy()
        .to_string()
        .replace('/', "_");

    home_dir()
        .expect("failed to get home directory")
        .join(CONFIG_LOCATION)
        .join(TERRAINS_DIR_NAME)
        .join(terrain_dir_name)
}

fn get_terrain_dir(cwd: &Path) -> Option<(PathBuf, PathBuf)> {
    if let Some(toml_path) = is_terrain_present(cwd) {
        return Some((cwd.to_path_buf(), toml_path));
    } else if cwd.parent().is_some() && cwd.parent().unwrap().exists() {
        return get_terrain_dir(cwd.parent().unwrap());
    }
    None
}

impl Context {
    pub fn get(home_dir: PathBuf, cwd: PathBuf) -> Result<Context> {
        let terrain_paths = get_terrain_dir(&cwd);

        if terrain_paths.is_none() {
            bail!("terrain.toml does not exists, run 'terrainium init' to initialize terrain.");
        }

        let (terrain_dir, toml_path) = terrain_paths.unwrap();
        let central_dir = get_central_dir_location(&terrain_dir);

        Self::generate(home_dir, terrain_dir, central_dir, toml_path)
    }

    pub fn create(home_dir: PathBuf, cwd: PathBuf, central: bool) -> Result<Context> {
        let terrain_dir = cwd;
        let central_dir = get_central_dir_location(&terrain_dir);

        if terrain_dir.join(TERRAIN_TOML).exists() || central_dir.join(TERRAIN_TOML).exists() {
            bail!(
                "terrain for this project is already present. edit existing terrain with 'terrain edit' command"
            );
        }

        let toml_path = if central {
            central_dir.join(TERRAIN_TOML)
        } else {
            terrain_dir.join(TERRAIN_TOML)
        };
        Self::generate(home_dir, terrain_dir, central_dir, toml_path)
    }

    fn generate(
        home_dir: PathBuf,
        terrain_dir: PathBuf,
        central_dir: PathBuf,
        toml_path: PathBuf,
    ) -> Result<Context> {
        let session_id = env::var(TERRAIN_SESSION_ID).ok();
        let config = Config::from_file().unwrap_or_default();

        let shell = Zsh::get(&current_dir().context("failed to get current directory")?);

        shell
            .setup_integration(Self::config_dir(home_dir).join(SHELL_INTEGRATION_SCRIPTS_DIR))
            .context("failed to setup shell integration")?;

        Ok(Context {
            session_id,
            central_dir,
            terrain_dir,
            toml_path,
            config,
            shell,
        })
    }

    pub fn session_id(&self) -> Option<String> {
        self.session_id.clone()
    }

    pub fn terrain_dir(&self) -> &Path {
        &self.terrain_dir
    }

    pub fn central_dir(&self) -> &Path {
        &self.central_dir
    }

    pub fn config_dir(home_dir: PathBuf) -> PathBuf {
        home_dir.join(CONFIG_LOCATION)
    }

    pub fn toml_path(&self) -> &Path {
        &self.toml_path
    }

    pub fn scripts_dir(&self) -> PathBuf {
        self.central_dir.join(SCRIPTS_DIR_NAME)
    }

    pub(crate) fn shell(&self) -> &Zsh {
        &self.shell
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn set_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    #[cfg(test)]
    pub(crate) fn build(
        terrain_dir: PathBuf,
        central_dir: PathBuf,
        toml_path: PathBuf,
        shell: Zsh,
    ) -> Self {
        Context {
            session_id: Some("some".to_string()),
            terrain_dir,
            central_dir,
            toml_path,
            config: Config::default(),
            shell,
        }
    }

    #[cfg(test)]
    pub(crate) fn build_with_config(
        terrain_dir: PathBuf,
        central_dir: PathBuf,
        toml_path: PathBuf,
        config: Config,
        shell: Zsh,
    ) -> Self {
        Context {
            session_id: Some("some".to_string()),
            terrain_dir,
            toml_path,
            central_dir,
            config,
            shell,
        }
    }

    #[cfg(test)]
    pub(crate) fn build_without_paths(shell: Zsh) -> Self {
        let terrain_dir = current_dir().expect("failed to get current directory");
        let toml_path = terrain_dir.join(TERRAIN_TOML);
        let central_dir = get_central_dir_location(&terrain_dir);
        Context {
            session_id: Some("some".to_string()),
            central_dir,
            terrain_dir,
            toml_path,
            config: Config::default(),
            shell,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::Context;
    use crate::client::shell::Zsh;
    use crate::client::utils::ExpectShell;
    use crate::common::execute::MockCommandToRun;
    use anyhow::Result;
    use home::home_dir;
    use serial_test::serial;
    use std::env::current_dir;
    use std::fs::{create_dir_all, write};
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    #[serial]
    #[test]
    fn creates_terrain_dir_context() -> Result<()> {
        let home_dir = tempdir()?;
        let terrain_dir = tempdir()?;
        let central_dir = get_central_dir_location(terrain_dir.path());

        let shell_integration_dir = get_shell_integration_dir(home_dir.path());

        let shell = MockCommandToRun::new_context();
        shell
            .expect()
            .withf(move |exe, args, env, cwd| {
                exe == "/bin/zsh"
                    && args.is_empty()
                    && env.is_none()
                    && cwd == current_dir().unwrap()
            })
            .returning(move |_, _, _, _| {
                let runner = MockCommandToRun::default();

                ExpectShell::with(runner)
                    .compile_script_for(
                        &shell_integration_dir.join("terrainium_init.zsh"),
                        &shell_integration_dir.join("terrainium_init.zsh.zwc"),
                    )
                    .successfully()
            });

        let context = Context::create(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
            false,
        )?;

        assert_eq!(terrain_dir.path(), context.terrain_dir());
        assert_eq!(central_dir, context.central_dir());
        assert_eq!(terrain_dir.path().join("terrain.toml"), context.toml_path());

        Ok(())
    }

    #[serial]
    #[test]
    fn creates_central_dir_context() -> Result<()> {
        let terrain_dir = tempdir()?;
        let central_dir = get_central_dir_location(terrain_dir.path());

        let home_dir = tempdir()?;
        let shell_integration_dir = get_shell_integration_dir(home_dir.path());

        let shell = MockCommandToRun::new_context();
        shell
            .expect()
            .withf(move |exe, args, env, cwd| {
                exe == "/bin/zsh"
                    && args.is_empty()
                    && env.is_none()
                    && cwd == current_dir().unwrap()
            })
            .returning(move |_, _, _, _| {
                let runner = MockCommandToRun::default();

                ExpectShell::with(runner)
                    .compile_script_for(
                        &shell_integration_dir.join("terrainium_init.zsh"),
                        &shell_integration_dir.join("terrainium_init.zsh.zwc"),
                    )
                    .successfully()
            });

        let context = Context::create(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
            true,
        )?;

        assert_eq!(terrain_dir.path(), context.terrain_dir());
        assert_eq!(central_dir, context.central_dir());
        assert_eq!(central_dir.join("terrain.toml"), context.toml_path());

        Ok(())
    }

    #[test]
    fn create_in_terrain_throws_error_if_already_present_in_terrain() -> Result<()> {
        let home_dir = tempdir()?;
        let terrain_dir = tempdir()?;
        write(terrain_dir.path().join("terrain.toml"), "")?;

        let err = Context::create(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
            false,
        )
        .expect_err("expected error")
        .to_string();

        assert_eq!(err, "terrain for this project is already present. edit existing terrain with 'terrain edit' command");

        Ok(())
    }

    #[test]
    fn create_in_terrain_throws_error_if_already_present_in_central() -> Result<()> {
        let home_dir = tempdir()?;
        let terrain_dir = tempdir()?;
        let central_dir = get_central_dir_location(terrain_dir.path());

        create_dir_all(&central_dir)?;
        write(central_dir.join("terrain.toml"), "")?;

        let err = Context::create(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
            false,
        )
        .expect_err("expected error")
        .to_string();

        assert_eq!(err, "terrain for this project is already present. edit existing terrain with 'terrain edit' command");

        Ok(())
    }

    #[test]
    fn create_in_central_throws_error_if_already_present_in_terrain() -> Result<()> {
        let home_dir = tempdir()?;
        let terrain_dir = tempdir()?;
        write(terrain_dir.path().join("terrain.toml"), "")?;

        let err = Context::create(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
            true,
        )
        .expect_err("expected error")
        .to_string();

        assert_eq!(err, "terrain for this project is already present. edit existing terrain with 'terrain edit' command");

        Ok(())
    }

    #[test]
    fn create_in_central_throws_error_if_already_present_in_central() -> Result<()> {
        let home_dir = tempdir()?;
        let terrain_dir = tempdir()?;
        let central_dir = get_central_dir_location(terrain_dir.path());
        create_dir_all(&central_dir)?;
        write(central_dir.join("terrain.toml"), "")?;

        let err = Context::create(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
            true,
        )
        .expect_err("expected error")
        .to_string();

        assert_eq!(err, "terrain for this project is already present. edit existing terrain with 'terrain edit' command");

        Ok(())
    }

    #[serial]
    #[test]
    fn get_in_terrain_dir() -> Result<()> {
        let home_dir = tempdir()?;
        let terrain_dir = tempdir()?;
        let central_dir = get_central_dir_location(terrain_dir.path());
        write(terrain_dir.path().join("terrain.toml"), "")?;

        let shell_integration_dir = get_shell_integration_dir(home_dir.path());

        let shell = MockCommandToRun::new_context();
        shell
            .expect()
            .withf(move |exe, args, env, cwd| {
                exe == "/bin/zsh"
                    && args.is_empty()
                    && env.is_none()
                    && cwd == current_dir().unwrap()
            })
            .returning(move |_, _, _, _| {
                let runner = MockCommandToRun::default();

                ExpectShell::with(runner)
                    .compile_script_for(
                        &shell_integration_dir.join("terrainium_init.zsh"),
                        &shell_integration_dir.join("terrainium_init.zsh.zwc"),
                    )
                    .successfully()
            });

        let context = Context::get(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
        )?;

        assert_eq!(terrain_dir.path(), context.terrain_dir());
        assert_eq!(central_dir, context.central_dir());
        assert_eq!(terrain_dir.path().join("terrain.toml"), context.toml_path());

        Ok(())
    }

    #[serial]
    #[test]
    fn get_in_central_dir() -> Result<()> {
        let home_dir = tempdir()?;
        let terrain_dir = tempdir()?;
        let central_dir = get_central_dir_location(terrain_dir.path());
        create_dir_all(&central_dir)?;
        write(central_dir.join("terrain.toml"), "")?;

        let shell_integration_dir = get_shell_integration_dir(home_dir.path());

        let shell = MockCommandToRun::new_context();
        shell
            .expect()
            .withf(move |exe, args, env, cwd| {
                exe == "/bin/zsh"
                    && args.is_empty()
                    && env.is_none()
                    && cwd == current_dir().unwrap()
            })
            .returning(move |_, _, _, _| {
                let runner = MockCommandToRun::default();

                ExpectShell::with(runner)
                    .compile_script_for(
                        &shell_integration_dir.join("terrainium_init.zsh"),
                        &shell_integration_dir.join("terrainium_init.zsh.zwc"),
                    )
                    .successfully()
            });

        let context = Context::get(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
        )?;

        assert_eq!(terrain_dir.path(), context.terrain_dir());
        assert_eq!(central_dir, context.central_dir());
        assert_eq!(central_dir.join("terrain.toml"), context.toml_path());

        Ok(())
    }

    #[serial]
    #[test]
    fn get_in_parent_terrain_dir() -> Result<()> {
        let home_dir = tempdir()?;
        let terrain_dir = tempdir()?;
        let cwd = terrain_dir.path().join("grand/child");
        create_dir_all(&cwd)?;
        let central_dir = get_central_dir_location(terrain_dir.path());
        write(terrain_dir.path().join("terrain.toml"), "")?;

        let shell_integration_dir = get_shell_integration_dir(home_dir.path());

        let shell = MockCommandToRun::new_context();
        shell
            .expect()
            .withf(move |exe, args, env, cwd| {
                exe == "/bin/zsh"
                    && args.is_empty()
                    && env.is_none()
                    && cwd == current_dir().unwrap()
            })
            .returning(move |_, _, _, _| {
                let runner = MockCommandToRun::default();

                ExpectShell::with(runner)
                    .compile_script_for(
                        &shell_integration_dir.join("terrainium_init.zsh"),
                        &shell_integration_dir.join("terrainium_init.zsh.zwc"),
                    )
                    .successfully()
            });

        let context = Context::get(home_dir.path().to_path_buf(), cwd)?;

        assert_eq!(terrain_dir.path(), context.terrain_dir());
        assert_eq!(central_dir, context.central_dir());
        assert_eq!(terrain_dir.path().join("terrain.toml"), context.toml_path());

        Ok(())
    }

    #[serial]
    #[test]
    fn get_in_parent_central_dir() -> Result<()> {
        let home_dir = tempdir()?;
        let terrain_dir = tempdir()?;
        let cwd = terrain_dir.path().join("grand/child");
        create_dir_all(&cwd)?;
        let central_dir = get_central_dir_location(terrain_dir.path());
        create_dir_all(&central_dir)?;
        write(central_dir.join("terrain.toml"), "")?;

        let shell_integration_dir = get_shell_integration_dir(home_dir.path());

        let shell = MockCommandToRun::new_context();
        shell
            .expect()
            .withf(move |exe, args, env, cwd| {
                exe == "/bin/zsh"
                    && args.is_empty()
                    && env.is_none()
                    && cwd == current_dir().unwrap()
            })
            .returning(move |_, _, _, _| {
                let runner = MockCommandToRun::default();

                ExpectShell::with(runner)
                    .compile_script_for(
                        &shell_integration_dir.join("terrainium_init.zsh"),
                        &shell_integration_dir.join("terrainium_init.zsh.zwc"),
                    )
                    .successfully()
            });

        let context = Context::get(home_dir.path().to_path_buf(), cwd)?;

        assert_eq!(terrain_dir.path(), context.terrain_dir());
        assert_eq!(central_dir, context.central_dir());
        assert_eq!(central_dir.join("terrain.toml"), context.toml_path());

        Ok(())
    }

    #[test]
    fn get_throws_error_if_not_present() -> Result<()> {
        let home_dir = tempdir()?;
        let terrain_dir = tempdir()?;
        let central_dir = get_central_dir_location(terrain_dir.path());
        create_dir_all(&central_dir)?;

        let err = Context::get(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
        )
        .expect_err("expected error")
        .to_string();

        assert_eq!(
            err,
            "terrain.toml does not exists, run 'terrainium init' to initialize terrain."
        );

        Ok(())
    }

    #[test]
    fn central_dir_returns_config_location() -> Result<()> {
        let context = Context::build_without_paths(Zsh::build(MockCommandToRun::default()));
        let central_dir = get_central_dir_location(&current_dir()?);

        assert_eq!(&central_dir, context.central_dir());
        Ok(())
    }

    #[test]
    fn scripts_dir_returns_scripts_location() -> Result<()> {
        let context = Context::build_without_paths(Zsh::build(MockCommandToRun::default()));
        let scripts_dir = get_central_dir_location(&current_dir()?).join("scripts");

        assert_eq!(scripts_dir, context.scripts_dir());
        Ok(())
    }

    pub(crate) fn get_shell_integration_dir(home_dir: &Path) -> PathBuf {
        home_dir.join(".config/terrainium/shell_integration")
    }

    fn get_central_dir_location(current_dir: &Path) -> PathBuf {
        let terrain_dir_name = Path::canonicalize(current_dir)
            .expect("current directory to be present")
            .to_string_lossy()
            .to_string()
            .replace('/', "_");

        home_dir()
            .expect("failed to get home directory")
            .join(".config/terrainium/terrains")
            .join(terrain_dir_name)
    }
}
