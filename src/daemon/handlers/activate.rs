use std::path::PathBuf;

use crate::{
    daemon::types::status::Status,
    helpers::operations::{create_dir_if_not_exist, fs},
    proto::{ActivateRequest, ActivateResponse},
};
use anyhow::{Context, Result};

pub fn handle(request: ActivateRequest) -> Result<ActivateResponse> {
    let mut tmp_dir = PathBuf::from(format!(
        "/tmp/terrain-{}-{}-{}",
        request.terrain_name.clone(),
        request.biome_name.clone(),
        request.session_id.clone()
    ));
    create_dir_if_not_exist(&tmp_dir).context("failed to create terrain temporary directory")?;

    tmp_dir.push("status.json");

    fs::write_file(
        &tmp_dir,
        serde_json::to_string_pretty(&Status::from(request))?,
    )?;

    Ok(ActivateResponse {})
}
