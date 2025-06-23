use clap::{Parser, Subcommand};
use tracing::Level;

#[derive(Parser, Debug)]
#[command()]
pub struct DaemonArgs {
    #[clap(flatten)]
    pub options: Options,

    #[command(subcommand)]
    pub verbs: Option<Verbs>,
}

#[derive(Parser, Debug)]
pub struct Options {
    #[arg(short, long)]
    pub force: bool,
    #[arg(short, long, default_value = "info")]
    pub log_level: Level,
    #[arg(long)]
    pub create_config: bool,
}

#[derive(Subcommand, Debug)]
pub enum Verbs {
    InstallService,
    RemoveService,
    EnableService {
        #[arg(short, long)]
        now: bool,
    },
    DisableService {
        #[arg(short, long)]
        now: bool,
    },
    StartService,
    StopService,
    Status,
}
