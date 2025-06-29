use anyhow::{Context, Result};
use clap::Parser;
use home::home_dir;
use terrainium_lib::styles::warning;

use crate::args::ClientArgs;
use crate::logging::init_logging;
use crate::shell::update_rc;

mod args;
mod constants;
mod context;
mod logging;
mod shell;
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
                    .context("failed to update shell rc file")?;
            }
        }
        Some(_) => {}
    }

    Ok(())
}
