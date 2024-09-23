use anyhow::{Context as AnyhowContext, Result};
use clap::Parser;
use terrainium::client::args::{ClientArgs, Commands};
use terrainium::client::handlers::{edit, init};
use terrainium::client::types::context::Context;

fn main() -> Result<()> {
    let args = ClientArgs::parse();
    let context = Context::generate();

    match args.command {
        Commands::Init { central, example } => {
            init::handle(context, central, example).context("failed to initialize new terrain")?
        }
        Commands::Edit => edit::handle(context).context("failed to edit terrain")?,
    }

    Ok(())
}
