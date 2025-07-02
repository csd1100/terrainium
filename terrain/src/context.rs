use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context as _, Result, bail};
use terrainium_lib::constants::CONFIG_LOCATION;
use terrainium_lib::executor::Execute;

use crate::args::Verbs;
use crate::config::Config;
use crate::constants::TERRAIN_TOML;
use crate::shell::{Shell, get_shell};

const TERRAINS_DIR_NAME: &str = "terrains";
const SCRIPTS_DIR_NAME: &str = "scripts";
const SHELL_INTEGRATION_SCRIPTS_DIR: &str = "shell_integration";

/// Returns path for central directory for the terrain.
///
/// If terrain is defined for project in directory `/home/user/work/repos/terrainium`,
/// then the central directory returned will be
/// `/home/user/.config/terrainium/terrains/_home_user_work_repos_terrainium`.
fn get_central_dir_location(home_dir: &Path, terrain_dir: &Path) -> PathBuf {
    let terrain_dir_name = terrain_dir
        .canonicalize()
        .expect("expected current directory to be valid")
        .to_string_lossy()
        .to_string()
        .replace('/', "_");

    home_dir
        .join(CONFIG_LOCATION)
        .join(TERRAINS_DIR_NAME)
        .join(terrain_dir_name)
}

/// contains all the required common data for terrain
/// to operate
pub struct Context {
    /// Session ID of the active terrain if is `Some`
    session_id: Option<String>,
    /// Directory for which terrain is initialized
    terrain_dir: PathBuf,
    /// Central Directory location for the terrain
    central_dir: PathBuf,
    /// Path of the `terrain.toml`
    toml_path: PathBuf,
    /// `terrain` config
    config: Config,
    /// Executor that will execute the required commands
    executor: Arc<dyn Execute>,
    /// Shell using which terrain is to be constructed
    shell: Box<dyn Shell>,
}

impl Context {
    /// Get terrainium configuration directory location
    pub fn config_dir(home_dir: &Path) -> PathBuf {
        home_dir.join(CONFIG_LOCATION)
    }

    /// Get shell-integration scripts directory
    pub fn shell_integration_dir(home_dir: &Path) -> PathBuf {
        Self::config_dir(home_dir).join(SHELL_INTEGRATION_SCRIPTS_DIR)
    }

    /// Creates a new [Context] object based on the current directory and operation
    /// to perform
    pub fn new(
        verb: &Verbs,
        home_dir: PathBuf,
        current_dir: PathBuf,
        executor: Arc<dyn Execute>,
    ) -> Result<Self> {
        if let Verbs::Init { central, .. } = verb {
            // always create context using current directory for init
            return Self::create(home_dir, current_dir, executor, *central);
        }
        todo!()
    }

    /// Creates the [Context] required for the creation of a new terrain
    ///
    /// Will return [Err] if terrain already is initialized
    fn create(
        home_dir: PathBuf,
        cwd: PathBuf,
        executor: Arc<dyn Execute>,
        central: bool,
    ) -> Result<Self> {
        let terrain_dir = cwd;
        let central_dir = get_central_dir_location(&home_dir, &terrain_dir);

        if terrain_dir.join(TERRAIN_TOML).exists() || central_dir.join(TERRAIN_TOML).exists() {
            bail!(
                "terrain for this project is already present. edit the existing terrain with \
                 'terrain edit' command"
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

    /// Creates [Context] object with provided information
    fn generate(
        home_dir: PathBuf,
        terrain_dir: PathBuf,
        central_dir: PathBuf,
        toml_path: PathBuf,
        session_id: Option<String>,
        executor: Arc<dyn Execute>,
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

    /// Directory for which terrain is created
    pub fn terrain_dir(&self) -> &Path {
        &self.terrain_dir
    }

    /// Path where `terrain.toml` is stored
    pub fn toml_path(&self) -> &Path {
        &self.toml_path
    }

    /// Directory where shell scripts used by terrainium are stored
    pub fn scripts_dir(&self) -> PathBuf {
        self.central_dir.join(SCRIPTS_DIR_NAME)
    }
}
