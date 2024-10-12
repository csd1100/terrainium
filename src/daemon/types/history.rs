use crate::common::constants::TERRAINIUMD_TMP_DIR;
use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::io::{Read, Write};

pub(crate) fn add(terrain_name: &str, session_id: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .read(true)
        .append(false)
        .open(format!("{}/{}/history", TERRAINIUMD_TMP_DIR, terrain_name))
        .context("failed to open history")?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .context("failed to read history")?;

    let mut history = contents.split('\n').take(3).collect::<Vec<&str>>();
    if history.len() == 3 {
        history.remove(1);
    }
    history.push(session_id);
    assert!(history.len() <= 3);

    file.write_all(history.join("\n").as_bytes())
        .context("failed to write history")?;

    Ok(())
}
