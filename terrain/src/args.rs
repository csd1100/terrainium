use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueHint};
use terrainium_lib::version::VERSION;
use tracing::Level;

use crate::constants::{SHELL, UNSUPPORTED, ZSH, ZSHRC_PATH};

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
}
