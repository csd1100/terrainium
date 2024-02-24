use std::{
    fs::File,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use tracing::{event, span, Level, Span};

use crate::{
    daemon::types::{client_status::get_status_for_session, status::{status_from, status_to}},
    helpers::utils::fs::create_dir_if_not_exist,
    proto::{ActivateRequest, ActivateResponse},
};
use anyhow::{Context, Result};

pub fn handle(
    span: &Span,
    session_id: String,
    daemon_status_file: Arc<Mutex<File>>,
    request: ActivateRequest,
) -> Result<ActivateResponse> {
    let activate = span!(parent: span, Level::TRACE, "activate");
    let _enter = activate.enter();

    event!(Level::TRACE, "generating status");
    let mut session = get_status_for_session(session_id.clone(), request.clone());
    event!(Level::DEBUG, "generated status {:?}", &session);
    session.toml_path = request.toml_path.into();

    let mut tmp_dir = PathBuf::from(format!(
        "/tmp/terrain-{}-{}-{}",
        request.terrain_name.clone(),
        request.biome_name.clone(),
        session_id.clone(),
    ));

    event!(Level::INFO, "creating directory {:?}", &tmp_dir);
    create_dir_if_not_exist(&tmp_dir).context(format!(
        "failed to create terrain temporary directory for session {}",
        session_id
    ))?;
    tmp_dir.push("status.json");

    event!(Level::INFO, "creating status file {:?}", &tmp_dir);
    let status_file = File::create(&tmp_dir).context(format!(
        "failed to create terrain status file for session {}",
        session_id
    ))?;

    event!(Level::INFO, "writing status to a file {:?}", &tmp_dir);
    let res = serde_json::to_writer_pretty(status_file, &session);
    if let Err(err) = res {
        event!(Level::ERROR, "error while writing status to file {:?}", err);
        panic!("error while writing status to file {:?}", err);
    }

    event!(Level::INFO, "acquiring a lock on daemon status file");
    let mut daemon_status = status_from(daemon_status_file.clone())
        .context("error while getting daemon_status_file for activating a terrain")?;

    let active_terrain = crate::daemon::types::status::ActiveTerrain {
        name: session.terrain_name,
        biome: session.biome_name,
        toml: session.toml_path,
        status_file: tmp_dir,
    };
    event!(
        Level::INFO,
        "adding active terrain {:?} in daemon status file",
        &active_terrain
    );
    daemon_status
        .active_terrains
        .insert(session_id, active_terrain);

    status_to(daemon_status_file, daemon_status)?;

    event!(Level::TRACE, "sending response");
    Ok(ActivateResponse {})
}
