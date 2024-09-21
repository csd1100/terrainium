use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command()]
pub struct ClientArgs {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Init
}