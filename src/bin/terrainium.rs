use anyhow::{Context as AnyhowContext, Result};
use clap::Parser;
use terrainium::client::args::{ClientArgs, Commands, GetArgs};
use terrainium::client::handlers::{edit, generate, get, init};
use terrainium::client::types::context::Context;

fn main() -> Result<()> {
    let args = ClientArgs::parse();
    let context = Context::generate();

    match args.command {
        Commands::Init {
            central,
            example,
            edit,
        } => init::handle(context, central, example, edit)
            .context("failed to initialize new terrain")?,
        Commands::Edit => edit::handle(context).context("failed to edit the terrain")?,
        Commands::Generate => {
            generate::handle(context).context("failed to generate scripts for the terrain")?
        }
        Commands::Get {
            biome,
            aliases,
            envs,
            alias,
            env,
            constructors,
            destructors,
        } => get::handle(
            context,
            GetArgs {
                biome,
                aliases,
                envs,
                alias,
                env,
                constructors,
                destructors,
            },
        )
        .context("failed to get the terrain values")?,
    }

    Ok(())
}
