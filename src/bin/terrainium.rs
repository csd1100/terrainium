use anyhow::{Context as AnyhowContext, Result};
use clap::Parser;
use home::home_dir;
use terrainium::client::args::{ClientArgs, GetArgs, UpdateArgs, Verbs};
use terrainium::client::handlers::{
    construct, destruct, edit, enter, exit, generate, get, init, update, validate,
};
use terrainium::client::logging::init_logging;
use terrainium::client::types::context::Context;
use tracing::metadata::LevelFilter;

#[cfg(feature = "terrain-schema")]
use terrainium::client::handlers::schema;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ClientArgs::parse();
    // need to keep _out_guard in scope till program exits for logger to work
    let (subscriber, _out_guard) = init_logging(LevelFilter::from(args.options.log_level));

    if let Some(Verbs::Get { .. }) = &args.command {
        // do not print any logs for get command as output will be used by scripts
    } else {
        tracing::subscriber::set_global_default(subscriber)
            .expect("unable to set global subscriber");
    }

    let context = Context::generate(&home_dir().expect("to detect home directory"));

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

            Verbs::Validate { fix } => {
                validate::handle(context, fix);
            }

            Verbs::Construct { biome } => construct::handle(context, biome, None).await?,

            Verbs::Destruct { biome } => destruct::handle(context, biome, None).await?,

            Verbs::Enter { biome, auto_apply } => {
                enter::handle(context, biome, auto_apply, None).await?
            }

            Verbs::Exit => exit::handle(context, None).await?,

            #[cfg(feature = "terrain-schema")]
            Verbs::Schema => schema::handle().context("failed to generate schema")?,
        },
    }

    Ok(())
}
