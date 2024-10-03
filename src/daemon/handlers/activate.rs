use crate::common::constants::TERRAINIUMD_TMP_DIR;
use crate::common::types::pb;
use crate::common::types::pb::{ActivateRequest, ActivateResponse};
use crate::daemon::handlers::execute::execute;
use crate::daemon::handlers::RequestHandler;
use crate::daemon::types::terrain_state::TerrainState;
use anyhow::{Context, Result};
use prost_types::Any;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{event, Level};

pub(crate) struct ActivateHandler;

impl RequestHandler for ActivateHandler {
    async fn handle(request: Any) -> Any {
        event!(Level::INFO, "handling ActivateRequest");
        let act_request: Result<ActivateRequest> = request
            .to_msg()
            .context("failed to convert request to type ActivateRequest");

        event!(
            Level::DEBUG,
            "result of attempting to parse request: {:#?}",
            act_request
        );

        match act_request {
            Ok(request) => {
                let response = activate(request).await;
                match response {
                    Ok(response) => Any::from_msg(&response).expect("to be converted to Any"),
                    Err(e) => Any::from_msg(&pb::Error {
                        error_message: e.to_string(),
                    })
                    .expect("to be converted to Any"),
                }
            }
            Err(err) => {
                event!(Level::ERROR, "failed to parse the request {:#?}", err);
                Any::from_msg(&pb::Error {
                    error_message: err.to_string(),
                })
                .expect("to be converted to Any")
            }
        }
    }
}

pub fn get_state_dir_path(terrain_name: &str, timestamp: &str) -> String {
    format!("{}/{}/{}", TERRAINIUMD_TMP_DIR, terrain_name, timestamp)
}

pub fn get_state_file_path(terrain_name: &str, timestamp: &str) -> String {
    format!("{}/state.json", get_state_dir_path(terrain_name, timestamp))
}

pub async fn get_state_file(
    terrain_name: &str,
    timestamp: &str,
    read: bool,
    write: bool,
    create: bool,
) -> fs::File {
    let mut file_options = fs::File::options();

    if create {
        file_options.create(true).truncate(true).write(true);
    } else if read {
        file_options.read(true);
    } else if write {
        file_options.write(true).truncate(true).append(false);
    }

    file_options
        .open(get_state_file_path(terrain_name, timestamp))
        .await
        .expect("Failed to open file")
}

pub async fn get_terrain_state(terrain_name: &str, timestamp: &str) -> TerrainState {
    let mut json = String::new();
    let mut readable_state_file = get_state_file(terrain_name, timestamp, true, false, false).await;
    readable_state_file
        .read_to_string(&mut json)
        .await
        .expect("failed to read state file");
    TerrainState::from_json(&json).expect("state to be parsed from json")
}

async fn activate(request: ActivateRequest) -> Result<ActivateResponse> {
    let execute_request = request.clone().execute.expect("to have execute request");
    let state: TerrainState = request.into();

    if !fs::try_exists(get_state_file_path(state.terrain_name(), state.timestamp()))
        .await
        .expect("to check if state dir exists")
    {
        fs::create_dir_all(get_state_dir_path(state.terrain_name(), state.timestamp()))
            .await
            .expect("to create state dir");
    }

    {
        let mut state_file =
            get_state_file(state.terrain_name(), state.timestamp(), false, false, true).await;

        state_file
            .write_all(
                state
                    .to_json()
                    .expect("expected state to be serialized to json")
                    .as_ref(),
            )
            .await
            .expect("Failed to write state to file");
    }

    execute(execute_request, (true, state.timestamp().to_string())).await;

    Ok(ActivateResponse {})
}
