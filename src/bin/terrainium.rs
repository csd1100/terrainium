use anyhow::{Context as AnyhowContext, Result};
use clap::Parser;
use home::home_dir;
use std::env::current_dir;
use terrainium::client::args::{ClientArgs, GetArgs, UpdateArgs, Verbs};
#[cfg(feature = "terrain-schema")]
use terrainium::client::handlers::schema;
use terrainium::client::handlers::{
    construct, destruct, edit, enter, exit, generate, get, init, update,
};
use terrainium::client::logging::init_logging;
use terrainium::client::shell::{Shell, Zsh};
use terrainium::client::types::config::Config;
use terrainium::client::types::context::Context;
use terrainium::client::types::environment::Environment;
use terrainium::client::types::terrain::Terrain;
use tracing::metadata::LevelFilter;
use tracing::Level;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ClientArgs::parse();

    // need to keep _out_guard in scope till program exits for logger to work
    let (subscriber, _out_guard) = if matches!(args.command, Some(Verbs::Validate)) {
        // if validate show debug level logs
        init_logging(LevelFilter::from(Level::DEBUG))
    } else {
        init_logging(LevelFilter::from(args.options.log_level))
    };

    if !matches!(args.command, Some(Verbs::Get { debug: false, .. })) {
        // do not print any logs for get command as output will be used by scripts
        tracing::subscriber::set_global_default(subscriber)
            .expect("unable to set global subscriber");
    }

    match args.command {
        None => {
            if args.options.update_rc || args.options.update_rc_path.is_some() {
                Zsh::update_rc(args.options.update_rc_path).context("failed to update rc")?;
            } else if args.options.create_config {
                Config::create_file().context("failed to create config file")?;
            }
        }
        Some(verbs) => {
            let home_dir = home_dir().context("failed to get home directory")?;
            let current_dir = current_dir().context("failed to get current directory")?;

            if let Verbs::Init {
                central,
                example,
                edit,
            } = verbs
            {
                let context = Context::create(home_dir, current_dir, central)?;
                return init::handle(context, example, edit)
                    .context("failed to initialize new terrain");
            }

            let context = Context::get(home_dir, current_dir)?;
            let terrain = Terrain::get_validated_and_fixed_terrain(&context)?;

            match verbs {
                Verbs::Init { .. } => {
                    // no need to do anything as it is handled above
                }

                Verbs::Edit => edit::handle(context).context("failed to edit the terrain")?,

                Verbs::Generate => generate::handle(context, terrain)
                    .context("failed to generate scripts for the terrain")?,

                Verbs::Validate => {
                    // create environments to run environment validations inside `Environment::from`
                    Environment::from(&terrain, None, context.terrain_dir())?;
                    let res: Result<Vec<Environment>> = terrain
                        .biomes()
                        .iter()
                        .map(|(biome_name, _)| {
                            // create environments to run environment validations
                            Environment::from(
                                &terrain,
                                Some(biome_name.to_string()),
                                context.terrain_dir(),
                            )
                        })
                        .collect();
                    // propagate any errors found during creation of environment
                    res?;
                }

                Verbs::Get {
                    debug: _debug,
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
                    terrain,
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
                    terrain,
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

                Verbs::Construct { biome } => construct::handle(context, biome, terrain, None)
                    .await
                    .context("failed to run the constructors for terrain")?,

                Verbs::Destruct { biome } => destruct::handle(context, biome, terrain, None)
                    .await
                    .context("failed to run the destructor for terrain")?,

                Verbs::Enter { biome, auto_apply } => {
                    enter::handle(context, biome, terrain, auto_apply, None)
                        .await
                        .context("failed to run enter the terrain")?
                }

                Verbs::Exit => exit::handle(context, terrain, None)
                    .await
                    .context("failed to exit the terrain")?,

                #[cfg(feature = "terrain-schema")]
                Verbs::Schema => schema::handle().context("failed to generate schema")?,
            }
        }
    }

    Ok(())
}
