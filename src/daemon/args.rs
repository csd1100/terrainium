use clap::Parser;
use tracing::Level;

#[derive(Parser, Debug)]
#[command()]
pub struct DaemonArgs {
    #[arg(short, long)]
    pub force: bool,
    #[arg(short, long, default_value = "info")]
    pub log_level: Level,
    #[arg(long)]
    pub create_config: bool,
}
