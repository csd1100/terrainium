use std::path::PathBuf;

use anyhow::Result;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, Registry};

use crate::helpers::constants::TERRAINIUMD_TMP;
use crate::helpers::operations::create_dir_if_not_exist;

pub fn init_logger() -> Result<()> {
    create_dir_if_not_exist(&PathBuf::from(TERRAINIUMD_TMP))?;
    let appender = tracing_appender::rolling::hourly(TERRAINIUMD_TMP, "log");
    let stdout = fmt::layer();
    let file = tracing_subscriber::fmt::layer()
        .with_writer(appender)
        .with_target(false)
        .with_ansi(false);
    Registry::default().with(stdout).with(file).init();
    Ok(())
}
