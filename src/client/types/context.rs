use crate::client::shell::{Shell, Zsh};
use crate::client::types::config::Config;
use crate::common::constants::{
    CONFIG_LOCATION, SHELL_INTEGRATION_SCRIPTS_DIR, TERRAIN_SESSION_ID, TERRAIN_TOML,
};
#[mockall_double::double]
use crate::common::execute::Executor;
use anyhow::{bail, Context as AnyhowContext, Result};
use std::env;
use std::env::current_dir;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug)]
pub struct Context {
    session_id: Option<String>,
    terrain_dir: PathBuf,
    central_dir: PathBuf,
    toml_path: PathBuf,
    config: Config,
    executor: Arc<Executor>,
    shell: Zsh,
}

const TERRAINS_DIR_NAME: &str = "terrains";
const SCRIPTS_DIR_NAME: &str = "scripts";

fn is_terrain_present(home_dir: &Path, cwd: &Path) -> Option<PathBuf> {
    let local_toml = cwd.join(TERRAIN_TOML);
    let central_toml = get_central_dir_location(home_dir, cwd).join(TERRAIN_TOML);

    if local_toml.exists() {
        return Some(local_toml);
    } else if central_toml.exists() {
        return Some(central_toml);
    }

    None
}

fn get_central_dir_location(home_dir: &Path, terrain_dir: &Path) -> PathBuf {
    let terrain_dir_name = Path::canonicalize(terrain_dir)
        .expect("expected current directory to be valid")
        .to_string_lossy()
        .to_string()
        .replace('/', "_");

    home_dir
        .join(CONFIG_LOCATION)
        .join(TERRAINS_DIR_NAME)
        .join(terrain_dir_name)
}

fn get_terrain_dir(home_dir: &Path, cwd: &Path) -> Option<(PathBuf, PathBuf)> {
    if let Some(toml_path) = is_terrain_present(home_dir, cwd) {
        return Some((cwd.to_path_buf(), toml_path));
    } else if cwd.parent().is_some() && cwd.parent().unwrap().exists() {
        return get_terrain_dir(home_dir, cwd.parent().unwrap());
    }
    None
}

impl Context {
    pub fn get(home_dir: PathBuf, cwd: PathBuf, executor: Executor) -> Result<Context> {
        let terrain_paths = get_terrain_dir(&home_dir, &cwd);

        if terrain_paths.is_none() {
            bail!("terrain.toml does not exists, run 'terrainium init' to initialize terrain.");
        }

        let (terrain_dir, toml_path) = terrain_paths.unwrap();
        let central_dir = get_central_dir_location(&home_dir, &terrain_dir);

        Self::generate(home_dir, terrain_dir, central_dir, toml_path, executor)
    }

    pub fn create(
        home_dir: PathBuf,
        cwd: PathBuf,
        executor: Executor,
        central: bool,
    ) -> Result<Context> {
        let terrain_dir = cwd;
        let central_dir = get_central_dir_location(&home_dir, &terrain_dir);

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
        Self::generate(home_dir, terrain_dir, central_dir, toml_path, executor)
    }

    fn generate(
        home_dir: PathBuf,
        terrain_dir: PathBuf,
        central_dir: PathBuf,
        toml_path: PathBuf,
        executor: Executor,
    ) -> Result<Context> {
        let session_id = env::var(TERRAIN_SESSION_ID).ok();
        let config = Config::from_file().unwrap_or_default();
        #[allow(clippy::default_constructed_unit_structs)]
        let executor = Arc::new(executor);

        let shell = Zsh::get(
            &current_dir().context("failed to get current directory")?,
            executor.clone(),
        );

        shell
            .setup_integration(Self::config_dir(home_dir).join(SHELL_INTEGRATION_SCRIPTS_DIR))
            .context("failed to setup shell integration")?;

        Ok(Context {
            session_id,
            central_dir,
            terrain_dir,
            toml_path,
            config,
            executor,
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

    pub(crate) fn executor(&self) -> &Arc<Executor> {
        &self.executor
    }

    pub(crate) fn shell(&self) -> &Zsh {
        &self.shell
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn set_session_id(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }

    #[cfg(test)]
    pub(crate) fn build(
        terrain_dir: &Path,
        central_dir: &Path,
        is_central: bool,
        executor: Executor,
    ) -> Self {
        let executor = Arc::new(executor);
        let toml_path = if is_central {
            central_dir.join(TERRAIN_TOML)
        } else {
            terrain_dir.join(TERRAIN_TOML)
        };
        Context {
            session_id: None,
            terrain_dir: terrain_dir.to_path_buf(),
            central_dir: central_dir.to_path_buf(),
            toml_path,
            config: Config::default(),
            executor: executor.clone(),
            shell: Zsh::get(terrain_dir, executor),
        }
    }

    #[cfg(test)]
    pub(crate) fn build_with_config(config: Config) -> Self {
        let executor = Arc::new(Executor::default());
        Context {
            session_id: None,
            terrain_dir: PathBuf::new(),
            central_dir: PathBuf::new(),
            toml_path: PathBuf::new(),
            config,
            executor: executor.clone(),
            shell: Zsh::get(Path::new(""), executor),
        }
    }

    #[cfg(test)]
    pub(crate) fn build_without_paths(executor: Executor) -> Self {
        use home::home_dir;

        let terrain_dir = current_dir().expect("failed to get current directory");
        let toml_path = terrain_dir.join(TERRAIN_TOML);
        let central_dir = get_central_dir_location(home_dir().unwrap().as_path(), &terrain_dir);
        let executor = Arc::new(executor);

        Context {
            session_id: None,
            central_dir,
            terrain_dir: terrain_dir.clone(),
            toml_path,
            config: Config::default(),
            executor: executor.clone(),
            shell: Zsh::get(terrain_dir.as_path(), executor),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::Context;
    use crate::client::test_utils::assertions::zsh::ExpectZSH;
    use crate::common::constants::TERRAIN_TOML;
    use crate::common::execute::MockExecutor;
    use anyhow::Result;
    use home::home_dir;
    use std::env::current_dir;
    use std::fs::{create_dir_all, write};
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    #[test]
    fn creates_terrain_dir_context() -> Result<()> {
        let home_dir = tempdir()?;
        let terrain_dir = tempdir()?;

        let central_dir = get_central_dir_location(home_dir.path(), terrain_dir.path());
        let shell_integration_dir = get_shell_integration_dir(home_dir.path());

        let executor = ExpectZSH::with(MockExecutor::new(), &current_dir()?)
            .compile_script_successfully_for(
                &shell_integration_dir.join("terrainium_init.zsh"),
                &shell_integration_dir.join("terrainium_init.zsh.zwc"),
            )
            .successfully();

        let context = Context::create(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
            executor,
            false,
        )?;

        assert_eq!(terrain_dir.path(), context.terrain_dir());
        assert_eq!(central_dir, context.central_dir());
        assert_eq!(terrain_dir.path().join(TERRAIN_TOML), context.toml_path());

        Ok(())
    }

    #[test]
    fn creates_central_dir_context() -> Result<()> {
        let home_dir = tempdir()?;
        let terrain_dir = tempdir()?;

        let central_dir = get_central_dir_location(home_dir.path(), terrain_dir.path());
        let shell_integration_dir = get_shell_integration_dir(home_dir.path());

        let executor = ExpectZSH::with(MockExecutor::new(), &current_dir()?)
            .compile_script_successfully_for(
                &shell_integration_dir.join("terrainium_init.zsh"),
                &shell_integration_dir.join("terrainium_init.zsh.zwc"),
            )
            .successfully();

        let context = Context::create(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
            executor,
            true,
        )?;

        assert_eq!(terrain_dir.path(), context.terrain_dir());
        assert_eq!(central_dir, context.central_dir());
        assert_eq!(central_dir.join(TERRAIN_TOML), context.toml_path());

        Ok(())
    }

    #[test]
    fn create_in_terrain_throws_error_if_already_present_in_terrain() -> Result<()> {
        let home_dir = tempdir()?;
        let terrain_dir = tempdir()?;
        write(terrain_dir.path().join(TERRAIN_TOML), "")?;

        let err = Context::create(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
            MockExecutor::new(),
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
        let central_dir = get_central_dir_location(home_dir.path(), terrain_dir.path());

        create_dir_all(&central_dir)?;
        write(central_dir.join(TERRAIN_TOML), "")?;

        let err = Context::create(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
            MockExecutor::new(),
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
        write(terrain_dir.path().join(TERRAIN_TOML), "")?;

        let err = Context::create(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
            MockExecutor::new(),
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
        let central_dir = get_central_dir_location(home_dir.path(), terrain_dir.path());
        create_dir_all(&central_dir)?;
        write(central_dir.join(TERRAIN_TOML), "")?;

        let err = Context::create(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
            MockExecutor::new(),
            true,
        )
        .expect_err("expected error")
        .to_string();

        assert_eq!(err, "terrain for this project is already present. edit existing terrain with 'terrain edit' command");

        Ok(())
    }

    #[test]
    fn get_in_terrain_dir() -> Result<()> {
        let home_dir = tempdir()?;
        let terrain_dir = tempdir()?;

        let central_dir = get_central_dir_location(home_dir.path(), terrain_dir.path());
        write(terrain_dir.path().join(TERRAIN_TOML), "")?;

        let shell_integration_dir = get_shell_integration_dir(home_dir.path());

        let executor = ExpectZSH::with(MockExecutor::new(), &current_dir()?)
            .compile_script_successfully_for(
                &shell_integration_dir.join("terrainium_init.zsh"),
                &shell_integration_dir.join("terrainium_init.zsh.zwc"),
            )
            .successfully();

        let context = Context::get(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
            executor,
        )?;

        assert_eq!(terrain_dir.path(), context.terrain_dir());
        assert_eq!(central_dir, context.central_dir());
        assert_eq!(terrain_dir.path().join(TERRAIN_TOML), context.toml_path());

        Ok(())
    }

    #[test]
    fn get_in_central_dir() -> Result<()> {
        let home_dir = tempdir()?;
        let terrain_dir = tempdir()?;

        let central_dir = get_central_dir_location(home_dir.path(), terrain_dir.path());

        create_dir_all(&central_dir)?;
        write(central_dir.join(TERRAIN_TOML), "")?;

        let shell_integration_dir = get_shell_integration_dir(home_dir.path());

        let executor = ExpectZSH::with(MockExecutor::new(), &current_dir()?)
            .compile_script_successfully_for(
                &shell_integration_dir.join("terrainium_init.zsh"),
                &shell_integration_dir.join("terrainium_init.zsh.zwc"),
            )
            .successfully();
        let context = Context::get(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
            executor,
        )?;

        assert_eq!(terrain_dir.path(), context.terrain_dir());
        assert_eq!(central_dir, context.central_dir());
        assert_eq!(central_dir.join(TERRAIN_TOML), context.toml_path());

        Ok(())
    }

    #[test]
    fn get_in_parent_terrain_dir() -> Result<()> {
        let home_dir = tempdir()?;
        let terrain_dir = tempdir()?;

        let cwd = terrain_dir.path().join("grand/child");
        create_dir_all(&cwd)?;

        let central_dir = get_central_dir_location(home_dir.path(), terrain_dir.path());
        write(terrain_dir.path().join(TERRAIN_TOML), "")?;

        let shell_integration_dir = get_shell_integration_dir(home_dir.path());

        let executor = ExpectZSH::with(MockExecutor::new(), &current_dir()?)
            .compile_script_successfully_for(
                &shell_integration_dir.join("terrainium_init.zsh"),
                &shell_integration_dir.join("terrainium_init.zsh.zwc"),
            )
            .successfully();

        let context = Context::get(home_dir.path().to_path_buf(), cwd, executor)?;

        assert_eq!(terrain_dir.path(), context.terrain_dir());
        assert_eq!(central_dir, context.central_dir());
        assert_eq!(terrain_dir.path().join(TERRAIN_TOML), context.toml_path());

        Ok(())
    }

    #[test]
    fn get_in_parent_central_dir() -> Result<()> {
        let home_dir = tempdir()?;
        let terrain_dir = tempdir()?;

        let cwd = terrain_dir.path().join("grand/child");
        create_dir_all(&cwd)?;

        let central_dir = get_central_dir_location(home_dir.path(), terrain_dir.path());
        create_dir_all(&central_dir)?;
        write(central_dir.join(TERRAIN_TOML), "")?;

        let shell_integration_dir = get_shell_integration_dir(home_dir.path());

        let executor = ExpectZSH::with(MockExecutor::new(), &current_dir()?)
            .compile_script_successfully_for(
                &shell_integration_dir.join("terrainium_init.zsh"),
                &shell_integration_dir.join("terrainium_init.zsh.zwc"),
            )
            .successfully();

        let context = Context::get(home_dir.path().to_path_buf(), cwd, executor)?;

        assert_eq!(terrain_dir.path(), context.terrain_dir());
        assert_eq!(central_dir, context.central_dir());
        assert_eq!(central_dir.join(TERRAIN_TOML), context.toml_path());

        Ok(())
    }

    #[test]
    fn get_throws_error_if_not_present() -> Result<()> {
        let home_dir = tempdir()?;
        let terrain_dir = tempdir()?;

        let central_dir = get_central_dir_location(home_dir.path(), terrain_dir.path());
        create_dir_all(&central_dir)?;

        let err = Context::get(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
            MockExecutor::new(),
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
        let context = Context::build_without_paths(MockExecutor::new());
        let central_dir = get_central_dir_location(home_dir().unwrap().as_path(), &current_dir()?);

        assert_eq!(&central_dir, context.central_dir());
        Ok(())
    }

    #[test]
    fn scripts_dir_returns_scripts_location() -> Result<()> {
        let context = Context::build_without_paths(MockExecutor::new());
        let scripts_dir = get_central_dir_location(home_dir().unwrap().as_path(), &current_dir()?)
            .join("scripts");

        assert_eq!(scripts_dir, context.scripts_dir());
        Ok(())
    }

    pub(crate) fn get_shell_integration_dir(home_dir: &Path) -> PathBuf {
        home_dir.join(".config/terrainium/shell_integration")
    }

    fn get_central_dir_location(home_dir: &Path, current_dir: &Path) -> PathBuf {
        let terrain_dir_name = Path::canonicalize(current_dir)
            .expect("current directory to be present")
            .to_string_lossy()
            .to_string()
            .replace('/', "_");

        home_dir
            .join(".config/terrainium/terrains")
            .join(terrain_dir_name)
    }
}
