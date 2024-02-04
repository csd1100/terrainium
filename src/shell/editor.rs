use std::path::PathBuf;

use anyhow::{Context, Result};

use super::execute::spawn_and_wait;

pub fn edit_file(file: PathBuf) -> Result<()> {
    let editor = std::env::var("EDITOR")
        .context("environment variable EDITOR not defined to edit terrain.")?;

    let file = file.to_str().expect("filepath to be converted to string");

    spawn_and_wait(&editor, vec![file], None)?;

    return Ok(());
}
