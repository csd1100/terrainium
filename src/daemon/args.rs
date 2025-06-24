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
    /// kills existing daemon and starts a new daemon
    #[arg(short, long, conflicts_with = "create_config")]
    pub force: bool,

    /// log level for daemon. allowed values: "trace", "debug", "info", "warn", "error"
    #[arg(short, long, default_value = "info")]
    pub log_level: Level,

    /// creates the daemon config at location ~/.config/terrainium/terrainiumd.toml
    #[arg(long)]
    pub create_config: bool,

    /// starts the daemon, will fail if daemon is already running
    #[arg(long, conflicts_with = "create_config")]
    pub run: bool,
}

#[derive(Subcommand, Debug)]
pub enum Verbs {
    /// installs the terrainiumd as a service and enables, starts the installed service
    Install,

    /// removes the terrainiumd as a service and stops the installed service
    Remove,

    /// enables terrainiumd service to be started on the machine startup
    Enable {
        /// start the terrainiumd process now if not running
        #[arg(short, long)]
        now: bool,
    },
    /// disables terrainiumd service to be started on the machine startup
    Disable {
        /// stop the terrainiumd process now if running
        #[arg(short, long)]
        now: bool,
    },

    /// start the terrainiumd process now if not running
    Start,

    /// stop the terrainiumd process now if running
    Stop,

    /// just reloads the service in the system (launchd, systemd).
    /// Does NOT start the service.
    Reload,

    /// prints status of the installed service, status can be: "running",
    /// "not running", "not loaded", "not installed"
    Status,
}
