use crate::common::constants::TERRAINIUMD_TMP_DIR;
use crate::common::types::pb;
use anyhow::{anyhow, Context, Result};
use std::fs::{create_dir_all, exists, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use tracing::{event, instrument, Level};

#[instrument]
pub(crate) fn add(terrain_name: &str, session_id: &str) -> Result<()> {
    event!(Level::DEBUG, "adding entry to history");
    create_dir_all(format!("{}/{}", TERRAINIUMD_TMP_DIR, terrain_name))
        .context("failed to create terrains directory")?;

    let history_path = format!("{}/{}/history", TERRAINIUMD_TMP_DIR, terrain_name);
    event!(
        Level::DEBUG,
        "generated history file path: {}",
        history_path
    );

    if !exists(&history_path).context(format!(
        "failed to check existence of history for terrain {}",
        terrain_name
    ))? {
        event!(
            Level::DEBUG,
            "history file: {} does not exist",
            history_path
        );
        create_history(&history_path, session_id)?;
    } else {
        event!(Level::DEBUG, "history file: {} found", history_path);
        add_history(&history_path, session_id)?;
    }

    Ok(())
}

fn create_history(path: &str, session_id: &str) -> Result<()> {
    event!(Level::INFO, "creating history file: {}", path);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .context("failed to create history file")?;

    event!(Level::TRACE, "adding to history file: {}", session_id);
    file.write_all(session_id.as_bytes())
        .context("failed to write to newly created history file")?;
    event!(Level::DEBUG, "added to history file: {}", session_id);

    Ok(())
}

fn add_history(path: &str, session_id: &str) -> Result<()> {
    event!(Level::TRACE, "reading the history file: {}", path);
    let mut read_file = OpenOptions::new()
        .read(true)
        .open(path)
        .context("failed to open history")?;

    let mut contents = String::new();
    read_file
        .read_to_string(&mut contents)
        .context("failed to read history")?;
    event!(Level::DEBUG, "contents of the history file: {}", contents);

    let mut history = contents.split('\n').collect::<Vec<&str>>();

    if history.len() == 3 {
        let removed = history.remove(0);
        event!(
            Level::DEBUG,
            "maximum entries already present in history file removing value: {}",
            removed
        );
    }
    event!(
        Level::DEBUG,
        "pushing entry on history file: {}",
        session_id
    );
    history.push(session_id);
    assert!(history.len() <= 3);

    let mut write_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)
        .context("failed to open history")?;

    let contents = history.join("\n");
    event!(Level::DEBUG, "writing to history file: {}", contents);

    write_file
        .write_all(contents.as_bytes())
        .context("failed to write history")?;

    event!(Level::TRACE, "added {} to history file", session_id);

    Ok(())
}

pub(crate) fn get_from_history(request: pb::StatusRequest) -> Result<PathBuf> {
    let history_path = format!("{}/{}/history", TERRAINIUMD_TMP_DIR, &request.terrain_name);
    event!(Level::TRACE, "reading the history file: {}", history_path);

    let mut read_file = OpenOptions::new()
        .read(true)
        .open(history_path)
        .context("failed to open history")?;

    let mut contents = String::new();
    read_file
        .read_to_string(&mut contents)
        .context("failed to read history")?;

    event!(Level::DEBUG, "contents of the history file: {}", contents);

    let mut history = contents
        .split('\n')
        .take_while(|str| !str.is_empty())
        .collect::<Vec<&str>>();

    if history.is_empty() {
        event!(Level::INFO, "no entries found in the history file",);
        return Err(anyhow!("no entries found in history"));
    }
    event!(
        Level::DEBUG,
        "{} entries found in the history file",
        history.len()
    );

    event!(
        Level::DEBUG,
        "history parameter to fetch entry {:?}",
        request.history
    );
    let session_id = match request.history {
        // recent
        0 => {
            let session_id = history.pop().unwrap();
            event!(Level::DEBUG, "session id from history: {}", session_id);
            session_id
        }
        // recent - 1
        1 => {
            if history.len() < 2 {
                return Err(anyhow!("only one entry found in history"));
            };
            let idx = if history.len() == 2 { 0 } else { 1 };
            let session_id = history.remove(idx);
            event!(
                Level::DEBUG,
                "session id from history: {}, from line: {}",
                session_id,
                idx
            );
            session_id
        }
        // recent - 2
        2 => {
            if history.len() < 3 {
                return Err(anyhow!("only two entry found in history"));
            };
            let session_id = history.remove(0);
            event!(Level::DEBUG, "session id from history: {}", session_id,);
            session_id
        }
        _ => return Err(anyhow::anyhow!("invalid history request")),
    };
    event!(
        Level::DEBUG,
        "session id:{} from history parameter to fetch entry {:?}",
        session_id,
        request.history
    );

    let path = format!(
        "{}/{}/{}/state.json",
        TERRAINIUMD_TMP_DIR, request.terrain_name, session_id
    );

    event!(
        Level::INFO,
        "status file path:{} for session id:{} from history parameter to fetch entry {:?}",
        path,
        session_id,
        request.history
    );
    Ok(PathBuf::from(path))
}
