use anyhow::{Context as AnyhowContext, Result};
use clap::Parser;
use home::home_dir;
use terrainium::client::args::{ClientArgs, GetArgs, UpdateArgs, Verbs};
use terrainium::client::handlers::{
    construct, destruct, edit, enter, exit, generate, get, init, update,
};
use terrainium::client::logging::init_logging;
use terrainium::client::types::context::Context;
use tracing::metadata::LevelFilter;

#[cfg(feature = "terrain-schema")]
use terrainium::client::handlers::schema;
use terrainium::client::types::terrain::Terrain;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ClientArgs::parse();
    // need to keep _out_guard in scope till program exits for logger to work
    let (subscriber, _out_guard) = init_logging(LevelFilter::from(args.options.log_level));
    if !matches!(args.command, Some(Verbs::Get { .. })) {
        // do not print any logs for get command as output will be used by scripts
        tracing::subscriber::set_global_default(subscriber)
            .expect("unable to set global subscriber");
    }

    let context = Context::generate(&home_dir().expect("to detect home directory"));
    let mut terrain: Option<Terrain> = None;

    if !(matches!(args.command, Some(Verbs::Init { .. }))
        || matches!(args.command, Some(Verbs::Edit { .. })))
    {
        terrain = Some(Terrain::get_validated_and_fixed_terrain(&context)?);
    }

    match args.command {
        None => {
            if args.options.update_rc || args.options.update_rc_path.is_some() {
                context
                    .update_rc(args.options.update_rc_path)
                    .context("failed to update rc")?;
            }
        }
        Some(verbs) => match verbs {
            Verbs::Init {
                central,
                example,
                edit,
            } => init::handle(context, central, example, edit)
                .context("failed to initialize new terrain")?,

            Verbs::Edit => edit::handle(context).context("failed to edit the terrain")?,

            Verbs::Generate => generate::handle(context, terrain.unwrap())
                .context("failed to generate scripts for the terrain")?,

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
                terrain.unwrap(),
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
                terrain.unwrap(),
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

            Verbs::Construct { biome } => construct::handle(context, biome, terrain.unwrap(), None)
                .await
                .context("failed to run the constructors for terrain")?,

            Verbs::Destruct { biome } => destruct::handle(context, biome, terrain.unwrap(), None)
                .await
                .context("failed to run the destructor for terrain")?,

            Verbs::Enter { biome, auto_apply } => {
                enter::handle(context, biome, terrain.unwrap(), auto_apply, None)
                    .await
                    .context("failed to run enter the terrain")?
            }

            Verbs::Exit => exit::handle(context, terrain.unwrap(), None)
                .await
                .context("failed to exit the terrain")?,

            #[cfg(feature = "terrain-schema")]
            Verbs::Schema => schema::handle().context("failed to generate schema")?,
        },
    }

    Ok(())
}
