use std::path::{Path, PathBuf};

use terrainium_lib::constants::CONFIG_LOCATION;

const SHELL_INTEGRATION_SCRIPTS_DIR: &str = "shell_integration";

/// contains all the required common data for terrain
/// to operate
pub struct Context {
    // session_id: Option<String>,
    // terrain_dir: PathBuf,
    // central_dir: PathBuf,
    // toml_path: PathBuf,
    // config: Config,
    // executor: Arc<Executor>,
    // shell: Zsh,
}

impl Context {
    pub fn config_dir(home_dir: &Path) -> PathBuf {
        home_dir.join(CONFIG_LOCATION)
    }

    pub fn shell_integration_dir(home_dir: &Path) -> PathBuf {
        Self::config_dir(home_dir).join(SHELL_INTEGRATION_SCRIPTS_DIR)
    }
}
