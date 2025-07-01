// FIXME: remove #![allow(dead_code)]
#![allow(dead_code)]
use anyhow::{Context, Result, bail};
use clap::Parser;
use home::home_dir;
use terrainium_lib::styles::warning;
use tokio::runtime::Builder;

use crate::args::{ClientArgs, Verbs};
use crate::config::Config;
use crate::handlers::status;
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

fn main() -> Result<()> {
    if cfg!(debug_assertions) {
        println!(
            "{}: you are running debug build of terrain, which might cause some unwanted behavior.",
            warning("WARNING")
        );
    }
    let args = ClientArgs::parse();

    let _out_guard = init_logging(&args);

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
                let rt = Builder::new_current_thread()
                    .build()
                    .context("failed to create async runtime")?;

                rt.block_on(async move {
                    status::handle(json, terrain_name, session_id, recent, None)
                        .await
                        .context("failed to get the terrain status")
                })
            } else {
                Ok(())
            }
        }
    }
}
