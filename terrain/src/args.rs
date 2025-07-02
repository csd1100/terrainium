use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueHint};
use terrainium_lib::version::VERSION;
use tracing::Level;

use crate::constants::{SHELL, TERRAIN_NAME, UNSUPPORTED, ZSH, ZSHRC_PATH};

/// get default rc path for supported shells
/// if unsupported shell is found, send UNSUPPORTED. UNSUPPORTED
/// value will be handled inside [shell::get_shell method](crate::client::shell::get_shell)
fn get_default_shell_rc() -> &'static str {
    let shell = std::env::var(SHELL).ok();

    if shell.is_some_and(|s| s.contains(ZSH)) {
        return ZSHRC_PATH;
    }

    UNSUPPORTED
}

/// terrainium
///
/// A command-line utility for environment management
#[derive(Parser)]
#[command(
    version(VERSION),
    propagate_version(true),
    args_conflicts_with_subcommands = true
)]
pub struct ClientArgs {
    #[clap(flatten)]
    pub options: Options,

    #[command(subcommand)]
    pub command: Option<Verbs>,
}

#[derive(Parser)]
pub struct Options {
    /// Creates a configuration file for terrain client
    ///
    /// Location: `~/.config/terrainium/terrainium.toml`
    #[arg(long, conflicts_with = "update_rc")]
    pub create_config: bool,

    /// Adds shell integration to specified rc file
    /// If file is not specified `~/.zshrc` is updated
    #[arg(long,
        num_args = 0..=1,
        default_missing_value = get_default_shell_rc(),
        value_hint = ValueHint::FilePath)]
    pub update_rc: Option<PathBuf>,

    /// Set logging level for validation messages
    ///
    /// For `terrain validate` value is overwritten to debug
    ///
    /// [possible values: trace, debug, info, warn, error]
    #[arg(
        short,
        long,
        default_value = "warn",
        global = true,
        display_order = 100
    )]
    pub log_level: Level,
}

#[derive(Subcommand)]
pub enum Verbs {
    /// Initialize terrain in current directory
    ///
    /// Creates terrain.toml file
    Init {
        /// Creates terrain.toml in central directory.
        ///
        /// If current directory is /home/user/work/project, then
        /// terrain.toml file is created in
        /// ~/.config/terrainium/terrains/_home_user_work_project/.
        ///
        /// This is useful if user does not want to add terrain.toml
        /// to source control
        #[arg(short, long)]
        central: bool,

        /// Creates terrain.toml with example terrain included.
        #[arg(short = 'x', long)]
        example: bool,

        /// Opens terrain.toml in EDITOR after creation
        ///
        /// Launches editor defined in EDITOR environment variable.
        /// If EDITOR environment variable is not set, 'vi' will be used
        /// as editor.
        #[arg(short, long)]
        edit: bool,
    },

    /// Validates the terrain in current directory
    Validate {},

    /// Fetch the values of the environment for current directory
    ///
    /// If no arguments are provided fetches all the values.
    Get {
        /// Prints the terrain validation logs
        #[arg(long)]
        debug: bool,
    },

    /// Fetches status of background constructors and destructors from terrainium
    /// daemon
    ///
    /// Fetches status for specified terrain name and session.
    /// If both session_id and recent are not provided (and TERRAIN_SESSION_ID is not set)
    /// will fetch most recently updated session.
    Status {
        /// Terrain for which status is to be fetched
        ///
        /// Needs to be specified if terrain is not active.
        ///
        /// If terrain is active, and this value is not specified, then value
        /// is read from TERRAIN_NAME environment variable.
        #[arg(short, long, env = TERRAIN_NAME, hide_env_values = true)]
        terrain_name: String,

        /// Return status for session_id [env: TERRAIN_SESSION_ID]
        ///
        /// If not specified read from TERRAIN_SESSION_ID environment variable,
        /// which is set when terrain activates.
        #[arg(short, long)]
        session_id: Option<String>,

        /// Return last updated nth session
        ///
        /// Cannot be used with session_id
        #[arg(short, long, value_name = "N", conflicts_with = "session_id")]
        recent: Option<u32>,

        /// Return status in json format
        #[arg(short, long)]
        json: bool,
    },
}
