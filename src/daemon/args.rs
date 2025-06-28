use clap::{Parser, Subcommand};
use tracing::Level;

#[derive(Parser, Debug)]
#[command(args_conflicts_with_subcommands = true)]
pub struct DaemonArgs {
    #[clap(flatten)]
    pub options: Options,

    #[command(subcommand)]
    pub verbs: Option<Verbs>,
}

#[derive(Parser, Debug)]
pub struct Options {
    /// Starts the daemon
    ///
    /// Will FAIL, if daemon is already running
    #[arg(long, conflicts_with = "create_config")]
    pub run: bool,

    /// Kills existing daemon and starts a new daemon
    #[arg(short, long, conflicts_with = "create_config")]
    pub force: bool,

    /// Log level for daemon.
    ///
    /// [possible values: trace, debug, info, warn, error]
    #[arg(short, long, default_value = "info")]
    pub log_level: Level,

    /// Creates the daemon config
    ///
    /// Location: ~/.config/terrainium/terrainiumd.toml
    #[arg(long)]
    pub create_config: bool,
}

#[derive(Subcommand, Debug)]
pub enum Verbs {
    /// Installs the terrainiumd as a service
    ///
    /// Enables, Starts the installed service as well.
    Install,

    /// Removes the terrainiumd as a service
    ///
    /// Stops the installed service
    Remove,

    /// Enables terrainiumd service to be started on the machine startup
    Enable {
        /// Start the terrainiumd process now if not running
        #[arg(short, long)]
        now: bool,
    },

    /// Disables terrainiumd service to be started on the machine startup
    Disable {
        /// Stop the terrainiumd process now if running
        #[arg(short, long)]
        now: bool,
    },

    /// Start the terrainiumd process now if not running
    Start,

    /// Stop the terrainiumd process now if running
    Stop,

    /// Just reloads the service in the system (launchd, systemd).
    /// Does NOT start the service.
    Reload,

    /// Prints status of the installed service
    ///
    /// Status can be: "running(enabled|disabled)",
    /// "not running(enabled|disabled)", "not loaded", "not installed"
    Status,
}
