use anyhow::{Context as AnyhowContext, Result};
use clap::Parser;
use std::path::PathBuf;
use terrainium::client::args::{ClientArgs, Commands, GetArgs, UpdateArgs};
use terrainium::client::handlers::{construct, destruct, edit, generate, get, init, update};
use terrainium::client::types::client::Client;
use terrainium::client::types::context::Context;
use terrainium::common::constants::TERRAINIUMD_SOCKET;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ClientArgs::parse();
    let mut context = Context::generate(None);

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
        Commands::Update {
            set_default,
            biome,
            new,
            env,
            alias,
            backup,
        } => update::handle(
            context,
            UpdateArgs {
                set_default,
                biome,
                alias,
                env,
                new,
                backup,
            },
        )
        .context("failed to update the terrain values")?,
        Commands::Construct { biome } => {
            let client = Client::new(PathBuf::from(TERRAINIUMD_SOCKET)).await?;
            context.set_client(client);
            construct::handle(&mut context, biome).await?
        }
        Commands::Destruct { biome } => {
            let client = Client::new(PathBuf::from(TERRAINIUMD_SOCKET)).await?;
            context.set_client(client);
            destruct::handle(&mut context, biome).await?
        }
    }

    Ok(())
}
