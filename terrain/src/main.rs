// FIXME: remove #![allow(dead_code)]
#![allow(dead_code)]
use std::sync::Arc;

use anyhow::{Context as _, Result, bail};
use clap::Parser;
use home::home_dir;
use terrainium_lib::executor::Executor;
use tracing::warn;

use crate::args::{ClientArgs, Verbs};
use crate::config::Config;
use crate::context::Context;
use crate::handlers::{init, status};
use crate::logging::init_logging;
use crate::shell::update_rc;

mod args;
mod config;
mod constants;
mod context;
mod handlers;
mod logging;
mod shell;
#[cfg(test)]
mod test_helpers;
mod types;
mod validate;

#[tokio::main]
async fn main() -> Result<()> {
    let args = ClientArgs::parse();

    let _out_guard = init_logging(&args);
    if cfg!(debug_assertions) {
        warn!("you are running debug build of terrain, which might cause some unwanted behavior.",);
    }

    let home_dir = home_dir().context("failed to get home directory")?;

    match args.command {
        None => {
            if args.options.update_rc.is_some() {
                update_rc(home_dir.as_path(), args.options.update_rc)
                    .context("failed to update shell rc file")
            } else if args.options.create_config {
                Config::create_file().context("failed to create config file")
            } else {
                bail!("must pass argument or command, run with --help for more information.");
            }
        }
        Some(verb) => {
            if let Verbs::Status {
                json,
                recent,
                session_id,
                terrain_name,
            } = verb
            {
                status::handle(json, terrain_name, session_id, recent, None)
                    .await
                    .context("failed to get the terrain status")
            } else {
                let current_dir =
                    std::env::current_dir().context("failed to get current directory")?;
                let context = Context::new(&verb, home_dir, current_dir, Arc::new(Executor))?;

                if let Verbs::Init { example, edit, .. } = verb {
                    return init::handle(context, example, edit)
                        .context("failed to initialize new terrain");
                }

                Ok(())
            }
        }
    }
}
