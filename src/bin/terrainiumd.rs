use anyhow::Result;
use clap::Parser;
use tracing::{event, span, Level};

use terrainium::daemon::{
    self,
    args::{DaemonArgs, Verbs},
    logging::init_logger,
};

fn main() -> Result<()> {
    init_logger()?;
    let application_logger = span!(Level::TRACE, "terrainiumd");
    let _enterd = application_logger.enter();

    event!(Level::TRACE, "parsing arguments");
    let opts = DaemonArgs::parse();

    match opts.verbs {
        Verbs::Start => daemon::start::handle(&application_logger),
        Verbs::Stop => todo!(),
    }

    Ok(())
}
