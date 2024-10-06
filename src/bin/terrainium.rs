use anyhow::{Context as AnyhowContext, Result};
use clap::Parser;
use std::path::PathBuf;
use terrainium::client::args::{ClientArgs, GetArgs, UpdateArgs, Verbs};
use terrainium::client::handlers::{
    construct, destruct, edit, enter, exit, generate, get, init, update,
};
use terrainium::client::types::client::Client;
use terrainium::client::types::context::Context;
use terrainium::common::constants::TERRAINIUMD_SOCKET;

#[cfg(feature = "terrain-schema")]
use terrainium::client::handlers::schema;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ClientArgs::parse();
    let mut context = Context::generate(None);

    match args.command {
        Verbs::Init {
            central,
            example,
            edit,
        } => init::handle(context, central, example, edit)
            .context("failed to initialize new terrain")?,

        Verbs::Edit => edit::handle(context).context("failed to edit the terrain")?,

        Verbs::Generate => {
            generate::handle(context).context("failed to generate scripts for the terrain")?
        }

        Verbs::Get {
            biome,
            aliases,
            envs,
            alias,
            env,
            constructors,
            destructors,
            auto_apply,
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
                auto_apply,
            },
        )
        .context("failed to get the terrain values")?,

        Verbs::Update {
            set_default,
            biome,
            new,
            env,
            alias,
            auto_apply,
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
                auto_apply,
            },
        )
        .context("failed to update the terrain values")?,

        Verbs::Construct { biome } => {
            let client = Client::new(PathBuf::from(TERRAINIUMD_SOCKET)).await?;
            context.set_client(client);
            construct::handle(&mut context, biome).await?
        }

        Verbs::Destruct { biome } => {
            let client = Client::new(PathBuf::from(TERRAINIUMD_SOCKET)).await?;
            context.set_client(client);
            destruct::handle(&mut context, biome).await?
        }

        Verbs::Enter { biome, auto_apply } => {
            let client = Client::new(PathBuf::from(TERRAINIUMD_SOCKET)).await?;
            context.set_client(client);
            enter::handle(&mut context, biome, auto_apply).await?
        }

        Verbs::Exit => {
            let client = Client::new(PathBuf::from(TERRAINIUMD_SOCKET)).await?;
            context.set_client(client);
            exit::handle(&mut context).await?
        }

        #[cfg(feature = "terrain-schema")]
        Verbs::Schema => schema::handle().context("failed to generate schema")?,
    }

    Ok(())
}
