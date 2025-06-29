use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context as AnyhowContext, Result, bail};
use terrainium_lib::constants::CONFIG_LOCATION;

use crate::client::args::Verbs;
use crate::client::shell::{Shell, Zsh, get_shell};
use crate::client::types::config::Config;
use crate::common::constants::{TERRAIN_DIR, TERRAIN_SESSION_ID, TERRAIN_TOML};
#[mockall_double::double]
use crate::common::execute::Executor;

const SHELL_INTEGRATION_SCRIPTS_DIR: &str = "shell_integration";

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
    pub fn new(
        verb: &Verbs,
        home_dir: PathBuf,
        current_dir: PathBuf,
        executor: Arc<Executor>,
    ) -> Result<Self> {
        let (dir, session_id) = match verb {
            Verbs::Init { central, .. } => {
                // always create context using current directory for init
                return Self::create(home_dir, current_dir, executor, *central);
            }
            Verbs::Edit { active: true, .. }
            | Verbs::Update { active: true, .. }
            | Verbs::Generate { active: true, .. }
            | Verbs::Get { active: true, .. }
            | Verbs::Validate { active: true, .. }
            | Verbs::Construct { .. }
            | Verbs::Destruct { .. }
            | Verbs::Exit
            | Verbs::Status { .. } => {
                // for edit, update, generate, get if active flag is passed
                // use TERRAIN_DIR to create context
                // for exit, construct, destruct only run if terrain is active
                let terrain_dir = std::env::var(TERRAIN_DIR);
                let session_id = std::env::var(TERRAIN_SESSION_ID);

                if session_id.is_err() && terrain_dir.is_err() {
                    bail!("terrain should be active for this command to run...")
                }

                (
                    PathBuf::from(terrain_dir.expect("terrain_dir to be present")),
                    Some(session_id.expect("session_id to be present")),
                )
            }
            // in other cases use current directory to create context
            _ => (current_dir, None),
        };
        Self::get(home_dir, dir, session_id, executor)
    }

    fn get(
        home_dir: PathBuf,
        cwd: PathBuf,
        session_id: Option<String>,
        executor: Arc<Executor>,
    ) -> Result<Self> {
        let terrain_paths = get_terrain_dir(&home_dir, &cwd);

        if terrain_paths.is_none() {
            bail!("terrain.toml does not exists, run 'terrain init' to initialize terrain.");
        }

        let (terrain_dir, toml_path) = terrain_paths.unwrap();
        let central_dir = get_central_dir_location(&home_dir, &terrain_dir);

        Self::generate(
            home_dir,
            terrain_dir,
            central_dir,
            toml_path,
            session_id,
            executor,
        )
    }

    fn create(
        home_dir: PathBuf,
        cwd: PathBuf,
        executor: Arc<Executor>,
        central: bool,
    ) -> Result<Self> {
        let terrain_dir = cwd;
        let central_dir = get_central_dir_location(&home_dir, &terrain_dir);

        if terrain_dir.join(TERRAIN_TOML).exists() || central_dir.join(TERRAIN_TOML).exists() {
            bail!(
                "terrain for this project is already present. edit existing terrain with 'terrain \
                 edit' command"
            );
        }

        let toml_path = if central {
            central_dir.join(TERRAIN_TOML)
        } else {
            terrain_dir.join(TERRAIN_TOML)
        };

        Self::generate(
            home_dir,
            terrain_dir,
            central_dir,
            toml_path,
            None,
            executor,
        )
    }

    fn generate(
        home_dir: PathBuf,
        terrain_dir: PathBuf,
        central_dir: PathBuf,
        toml_path: PathBuf,
        session_id: Option<String>,
        executor: Arc<Executor>,
    ) -> Result<Self> {
        let config = Config::from_file().unwrap_or_default();

        let cwd = std::env::current_dir().context("failed to get current directory")?;
        let shell = get_shell(cwd.as_path(), executor.clone())?;

        shell
            .create_integration_script(
                Self::config_dir(home_dir.as_path()).join(SHELL_INTEGRATION_SCRIPTS_DIR),
            )
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

    pub fn config_dir(home_dir: &Path) -> PathBuf {
        home_dir.join(CONFIG_LOCATION)
    }

    pub fn shell_integration_dir(home_dir: &Path) -> PathBuf {
        Self::config_dir(home_dir).join(SHELL_INTEGRATION_SCRIPTS_DIR)
    }

    pub fn toml_path(&self) -> &Path {
        &self.toml_path
    }

    pub fn scripts_dir(&self) -> PathBuf {
        self.central_dir.join(SCRIPTS_DIR_NAME)
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub(crate) fn executor(&self) -> &Arc<Executor> {
        &self.executor
    }

    pub(crate) fn shell(&self) -> &Zsh {
        &self.shell
    }

    pub(crate) fn set_session_id(mut self, session_id: &str) -> Self {
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

        let terrain_dir = std::env::current_dir().expect("failed to get current directory");
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
    use std::env::{VarError, current_dir};
    use std::fs::{create_dir_all, write};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use anyhow::Result;
    use home::home_dir;
    use pretty_assertions::assert_eq;
    use serial_test::serial;
    use tempfile::tempdir;

    use super::Context;
    use crate::client::args::{BiomeArg, Verbs};
    use crate::client::shell::{Shell, Zsh};
    use crate::client::test_utils::assertions::zsh::ExpectZSH;
    use crate::client::test_utils::{restore_env_var, set_env_var};
    use crate::client::types::terrain::Terrain;
    use crate::common::constants::{TERRAIN_DIR, TERRAIN_SESSION_ID, TERRAIN_TOML};
    use crate::common::execute::MockExecutor;
    use crate::common::test_utils::TEST_SESSION_ID;

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

    #[test]
    fn creates_terrain_dir_context() -> Result<()> {
        let home_dir = tempdir()?;
        let terrain_dir = tempdir()?;

        let central_dir = get_central_dir_location(home_dir.path(), terrain_dir.path());
        let shell_integration_dir = get_shell_integration_dir(home_dir.path());

        let executor = ExpectZSH::with(MockExecutor::new(), &current_dir()?)
            .compile_script_successfully_for(
                &shell_integration_dir.join("terrainium_init.zsh"),
                &shell_integration_dir.join("terrainium_init.zwc"),
            )
            .successfully();

        let context = Context::create(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
            Arc::new(executor),
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
                &shell_integration_dir.join("terrainium_init.zwc"),
            )
            .successfully();

        let context = Context::create(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
            Arc::new(executor),
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
            Arc::new(MockExecutor::new()),
            false,
        )
        .expect_err("expected error")
        .to_string();

        assert_eq!(
            err,
            "terrain for this project is already present. edit existing terrain with 'terrain \
             edit' command"
        );

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
            Arc::new(MockExecutor::new()),
            false,
        )
        .expect_err("expected error")
        .to_string();

        assert_eq!(
            err,
            "terrain for this project is already present. edit existing terrain with 'terrain \
             edit' command"
        );

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
            Arc::new(MockExecutor::new()),
            true,
        )
        .expect_err("expected error")
        .to_string();

        assert_eq!(
            err,
            "terrain for this project is already present. edit existing terrain with 'terrain \
             edit' command"
        );

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
            Arc::new(MockExecutor::new()),
            true,
        )
        .expect_err("expected error")
        .to_string();

        assert_eq!(
            err,
            "terrain for this project is already present. edit existing terrain with 'terrain \
             edit' command"
        );

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
                &shell_integration_dir.join("terrainium_init.zwc"),
            )
            .successfully();

        let context = Context::get(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
            Some(TEST_SESSION_ID.to_string()),
            Arc::new(executor),
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
                &shell_integration_dir.join("terrainium_init.zwc"),
            )
            .successfully();
        let context = Context::get(
            home_dir.path().to_path_buf(),
            terrain_dir.path().to_path_buf(),
            Some(TEST_SESSION_ID.to_string()),
            Arc::new(executor),
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
                &shell_integration_dir.join("terrainium_init.zwc"),
            )
            .successfully();

        let context = Context::get(
            home_dir.path().to_path_buf(),
            cwd,
            Some(TEST_SESSION_ID.to_string()),
            Arc::new(executor),
        )?;

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
                &shell_integration_dir.join("terrainium_init.zwc"),
            )
            .successfully();

        let context = Context::get(
            home_dir.path().to_path_buf(),
            cwd,
            Some(TEST_SESSION_ID.to_string()),
            Arc::new(executor),
        )?;

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
            Some(TEST_SESSION_ID.to_string()),
            Arc::new(MockExecutor::new()),
        )
        .expect_err("expected error")
        .to_string();

        assert_eq!(
            err,
            "terrain.toml does not exists, run 'terrain init' to initialize terrain."
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

    #[serial]
    #[test]
    fn context_for_args() -> Result<()> {
        let home_dir = tempdir()?;
        let current_dir = tempdir()?;
        let terrain_directory = tempdir()?;
        let init_dir = tempdir()?;

        // create test terrain.toml
        write(
            current_dir.path().join(TERRAIN_TOML),
            toml::to_string_pretty(&Terrain::example())?,
        )?;
        write(
            terrain_directory.path().join(TERRAIN_TOML),
            toml::to_string_pretty(&Terrain::example())?,
        )?;

        let session_id: std::result::Result<String, VarError>;
        let terrain_dir: std::result::Result<String, VarError>;

        unsafe {
            session_id = set_env_var(TERRAIN_SESSION_ID, Some(TEST_SESSION_ID));
            terrain_dir = set_env_var(
                TERRAIN_DIR,
                Some(terrain_directory.path().to_str().unwrap()),
            )
        }

        let shell_integration_dir = get_shell_integration_dir(home_dir.path());
        let executor = ExpectZSH::with(MockExecutor::new(), &std::env::current_dir()?)
            .compile_script_successfully_for_times(
                &shell_integration_dir.join("terrainium_init.zsh"),
                &shell_integration_dir.join("terrainium_init.zwc"),
                16,
            )
            .successfully();

        let executor = Arc::new(executor);

        let current_dir_ctx = Context {
            session_id: None,
            terrain_dir: current_dir.path().to_path_buf(),
            central_dir: get_central_dir_location(home_dir.path(), current_dir.path()),
            toml_path: current_dir.path().join(TERRAIN_TOML),
            config: Default::default(),
            executor: executor.clone(),
            shell: Zsh::get(current_dir.path(), executor.clone()),
        };

        let init_dir_ctx = Context {
            session_id: None,
            terrain_dir: init_dir.path().to_path_buf(),
            central_dir: get_central_dir_location(home_dir.path(), init_dir.path()),
            toml_path: init_dir.path().join(TERRAIN_TOML),
            config: Default::default(),
            executor: executor.clone(),
            shell: Zsh::get(init_dir.path(), executor.clone()),
        };

        let central_dir_ctx = Context {
            session_id: None,
            terrain_dir: init_dir.path().to_path_buf(),
            central_dir: get_central_dir_location(home_dir.path(), init_dir.path()),
            toml_path: get_central_dir_location(home_dir.path(), init_dir.path())
                .join(TERRAIN_TOML),
            config: Default::default(),
            executor: executor.clone(),
            shell: Zsh::get(init_dir.path(), executor.clone()),
        };

        let terrain_dir_ctx = Context {
            session_id: Some(TEST_SESSION_ID.to_string()),
            terrain_dir: terrain_directory.path().to_path_buf(),
            central_dir: get_central_dir_location(home_dir.path(), terrain_directory.path()),
            toml_path: terrain_directory.path().join(TERRAIN_TOML),
            config: Default::default(),
            executor: executor.clone(),
            shell: Zsh::get(current_dir.path(), executor.clone()),
        };

        struct TestVerbContext<'a> {
            verb: Verbs,
            expected: &'a Context,
        }

        let verbs: Vec<TestVerbContext> = vec![
            TestVerbContext {
                verb: Verbs::Init {
                    central: false,
                    example: true,
                    edit: false,
                },
                expected: &init_dir_ctx,
            },
            TestVerbContext {
                verb: Verbs::Init {
                    central: true,
                    example: true,
                    edit: false,
                },
                expected: &central_dir_ctx,
            },
            TestVerbContext {
                verb: Verbs::Enter {
                    biome: BiomeArg::None,
                    auto_apply: false,
                },
                expected: &current_dir_ctx,
            },
            TestVerbContext {
                verb: Verbs::Validate { active: true },
                expected: &terrain_dir_ctx,
            },
            TestVerbContext {
                verb: Verbs::Validate { active: false },
                expected: &current_dir_ctx,
            },
            TestVerbContext {
                verb: Verbs::Edit { active: false },
                expected: &current_dir_ctx,
            },
            TestVerbContext {
                verb: Verbs::Generate { active: false },
                expected: &current_dir_ctx,
            },
            TestVerbContext {
                verb: Verbs::Get {
                    active: false,
                    debug: false,
                    json: false,
                    biome: BiomeArg::None,
                    aliases: false,
                    envs: false,
                    alias: vec![],
                    env: vec![],
                    constructors: false,
                    destructors: false,
                    auto_apply: false,
                },
                expected: &current_dir_ctx,
            },
            TestVerbContext {
                verb: Verbs::Update {
                    active: false,
                    set_default: None,
                    biome: BiomeArg::None,
                    new: None,
                    alias: vec![],
                    env: vec![],
                    auto_apply: None,
                    backup: false,
                },
                expected: &current_dir_ctx,
            },
            TestVerbContext {
                verb: Verbs::Edit { active: true },
                expected: &terrain_dir_ctx,
            },
            TestVerbContext {
                verb: Verbs::Generate { active: true },
                expected: &terrain_dir_ctx,
            },
            TestVerbContext {
                verb: Verbs::Get {
                    active: true,
                    debug: false,
                    json: false,
                    biome: BiomeArg::None,
                    aliases: false,
                    envs: false,
                    alias: vec![],
                    env: vec![],
                    constructors: false,
                    destructors: false,
                    auto_apply: false,
                },
                expected: &terrain_dir_ctx,
            },
            TestVerbContext {
                verb: Verbs::Update {
                    active: true,
                    set_default: None,
                    biome: BiomeArg::None,
                    new: None,
                    alias: vec![],
                    env: vec![],
                    auto_apply: None,
                    backup: false,
                },
                expected: &terrain_dir_ctx,
            },
            TestVerbContext {
                verb: Verbs::Construct {
                    biome: BiomeArg::None,
                },
                expected: &terrain_dir_ctx,
            },
            TestVerbContext {
                verb: Verbs::Destruct {
                    biome: BiomeArg::None,
                },
                expected: &terrain_dir_ctx,
            },
            TestVerbContext {
                verb: Verbs::Exit,
                expected: &terrain_dir_ctx,
            },
        ];

        for data in verbs {
            let dir = if matches!(data.verb, Verbs::Init { .. }) {
                init_dir.path().to_path_buf()
            } else {
                current_dir.path().to_path_buf()
            };

            let actual = Context::new(
                &data.verb,
                home_dir.path().to_path_buf(),
                dir,
                executor.clone(),
            );

            assert!(
                actual.is_ok(),
                "failed to create a new context for verb: {:?}",
                data.verb
            );

            let actual = actual.expect("to be present");

            assert_eq!(
                actual.session_id, data.expected.session_id,
                "failed to validate session id context creation for verb: {:?}",
                data.verb
            );
            assert_eq!(
                actual.terrain_dir, data.expected.terrain_dir,
                "failed to validate terrain dir context creation for verb: {:?}",
                data.verb
            );
            assert_eq!(
                actual.central_dir, data.expected.central_dir,
                "failed to validate central dir context creation for verb: {:?}",
                data.verb
            );
            assert_eq!(
                actual.toml_path, data.expected.toml_path,
                "failed to validate toml path context creation for verb: {:?}",
                data.verb
            );
        }

        unsafe {
            restore_env_var(TERRAIN_SESSION_ID, session_id);
            restore_env_var(TERRAIN_DIR, terrain_dir);
        }
        Ok(())
    }

    #[serial]
    #[test]
    fn context_for_args_errors() -> Result<()> {
        let temp_dir = tempdir()?;

        let verbs: Vec<Verbs> = vec![
            Verbs::Edit { active: true },
            Verbs::Generate { active: true },
            Verbs::Get {
                active: true,
                debug: false,
                json: false,
                biome: BiomeArg::None,
                aliases: false,
                envs: false,
                alias: vec![],
                env: vec![],
                constructors: false,
                destructors: false,
                auto_apply: false,
            },
            Verbs::Update {
                active: true,
                set_default: None,
                biome: BiomeArg::None,
                new: None,
                alias: vec![],
                env: vec![],
                auto_apply: None,
                backup: false,
            },
            Verbs::Construct {
                biome: BiomeArg::None,
            },
            Verbs::Destruct {
                biome: BiomeArg::None,
            },
            Verbs::Exit,
        ];

        let executor = Arc::new(MockExecutor::new());

        for verb in verbs {
            let actual = Context::new(
                &verb,
                temp_dir.path().to_path_buf(),
                temp_dir.path().to_path_buf(),
                executor.clone(),
            )
            .expect_err(&format!(
                "failed to get an error while creating context for verb: {verb:?}"
            ))
            .to_string();

            assert_eq!(
                actual, "terrain should be active for this command to run...",
                "failed to validate error message for context creation verb: {:?}",
                verb
            );
        }

        Ok(())
    }
}
