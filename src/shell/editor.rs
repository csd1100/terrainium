use std::path::PathBuf;

use anyhow::{Context, Result};

use super::execute::Execute;

pub fn edit_file(file: &PathBuf) -> Result<()> {
    let editor = std::env::var("EDITOR")
        .context("environment variable EDITOR not defined to edit terrain.")?;

    let file = file.to_str().expect("filepath to be converted to string");

    Execute::spawn_and_wait(&editor, vec![file], None)
        .context(format!("failed to start editor {}", editor))?;

    return Ok(());
}
