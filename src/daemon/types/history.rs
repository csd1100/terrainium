use crate::common::constants::TERRAINIUMD_TMP_DIR;
use anyhow::{Context, Result};
use std::fs::{create_dir_all, exists, OpenOptions};
use std::io::{Read, Write};

pub(crate) fn add(terrain_name: &str, session_id: &str) -> Result<()> {
    create_dir_all(format!("{}/{}", TERRAINIUMD_TMP_DIR, terrain_name))
        .context("failed to create terrains directory")?;
    let history_path = format!("{}/{}/history", TERRAINIUMD_TMP_DIR, terrain_name);

    if !exists(&history_path).context(format!(
        "failed to check existence of history for terrain {}",
        terrain_name
    ))? {
        create_history(&history_path, session_id)?;
    } else {
        add_history(&history_path, session_id)?;
    }

    Ok(())
}

fn create_history(path: &str, session_id: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .context("failed to create history file")?;

    file.write_all(session_id.as_bytes())
        .context("failed to write to newly created history file")?;

    Ok(())
}

fn add_history(path: &str, session_id: &str) -> Result<()> {
    let mut read_file = OpenOptions::new()
        .read(true)
        .open(path)
        .context("failed to open history")?;

    let mut contents = String::new();
    read_file
        .read_to_string(&mut contents)
        .context("failed to read history")?;

    let mut history = contents.split('\n').collect::<Vec<&str>>();

    if history.len() == 3 {
        history.remove(0);
    }
    history.push(session_id);
    assert!(history.len() <= 3);

    let mut write_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)
        .context("failed to open history")?;

    write_file
        .write_all(history.join("\n").as_bytes())
        .context("failed to write history")?;

    Ok(())
}
