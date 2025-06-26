use std::sync::Arc;

use anyhow::{Context as AnyhowContext, Result, bail};
use clap::Parser;
use home::home_dir;
use terrainium::client::args::{BiomeArg, ClientArgs, GetArgs, UpdateArgs, Verbs};
#[cfg(feature = "terrain-schema")]
use terrainium::client::handlers::schema;
use terrainium::client::handlers::{
    construct, destruct, edit, enter, exit, generate, get, init, status, update,
};
use terrainium::client::logging::init_logging;
use terrainium::client::shell::{Shell, Zsh};
use terrainium::client::types::config::Config;
use terrainium::client::types::context::Context;
use terrainium::client::types::environment::Environment;
use terrainium::client::types::terrain::Terrain;
use terrainium::common::execute::Executor;
use terrainium::common::types::styles::warning;

#[tokio::main]
async fn main() -> Result<()> {
    if cfg!(debug_assertions) {
        println!(
            "{}: you are running debug build of terrainium, which might cause some unwanted \
             behavior.",
            warning("WARNING")
        );
    }

    let args = ClientArgs::parse();
    let _out_guard = init_logging(&args);

    match args.command {
        None => {
            if args.options.update_rc || args.options.update_rc_path.is_some() {
                Zsh::update_rc(args.options.update_rc_path).context("failed to update rc")?;
            } else if args.options.create_config {
                Config::create_file().context("failed to create config file")?;
            } else {
                bail!("must pass argument or command, run with --help for more information.");
            }
        }
        Some(verbs) => {
            if let Verbs::Status {
                json,
                recent,
                session_id,
                terrain_name,
            } = verbs
            {
                return status::handle(json, terrain_name, session_id, recent, None)
                    .await
                    .context("failed to get the terrain status");
            }

            let home_dir = home_dir().context("failed to get home directory")?;
            let current_dir = std::env::current_dir().context("failed to get current directory")?;
            let context = Context::new(&verbs, home_dir, current_dir, Arc::new(Executor))?;

            if let Verbs::Init { example, edit, .. } = verbs {
                return init::handle(context, example, edit)
                    .context("failed to initialize new terrain");
            }

            if let Verbs::Edit { .. } = verbs {
                return edit::handle(context).context("failed to edit the terrain");
            }

            let (terrain, terrain_toml) = Terrain::get_validated_and_fixed_terrain(&context)?;

            match verbs {
                Verbs::Init { .. } | Verbs::Edit { .. } => {
                    // no need to do anything as it is handled above
                }

                Verbs::Generate { .. } => generate::handle(context, terrain)
                    .context("failed to generate scripts for the terrain")?,

                Verbs::Validate => {
                    // create environments to run environment validations inside `Environment::from`
                    Environment::from(&terrain, BiomeArg::None, context.terrain_dir())?;
                    let res: Result<Vec<Environment>> = terrain
                        .biomes()
                        .iter()
                        .map(|(biome_name, _)| {
                            // create environments to run environment validations
                            Environment::from(
                                &terrain,
                                BiomeArg::Some(biome_name.to_string()),
                                context.terrain_dir(),
                            )
                        })
                        .collect();
                    // propagate any errors found during creation of environment
                    res?;
                }

                Verbs::Get {
                    json,
                    biome,
                    aliases,
                    envs,
                    alias,
                    env,
                    constructors,
                    destructors,
                    auto_apply,
                    ..
                } => get::handle(
                    context,
                    terrain,
                    GetArgs {
                        json,
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
                    ..
                } => update::handle(
                    context,
                    terrain,
                    terrain_toml,
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
                        .context("failed to enter the terrain")?
                }

                Verbs::Exit => exit::handle(context, terrain, None)
                    .await
                    .context("failed to exit the terrain")?,

                Verbs::Status { .. } => {
                    // no need to do anything as handled above
                }

                #[cfg(feature = "terrain-schema")]
                Verbs::Schema => schema::handle().context("failed to generate schema")?,
            }
        }
    }

    Ok(())
}
