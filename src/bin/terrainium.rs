use anyhow::Result;
use clap::Parser;
use terrainium::client::args::{ClientArgs, Commands};
use terrainium::client::handlers::init;
use terrainium::client::types::context::Context;

fn main() -> Result<()> {
    let args = ClientArgs::parse();
    let context = Context::new();

    match args.command {
        Commands::Init {
            central
        } => init::handle(context, central)?,
    }

    Ok(())
}
