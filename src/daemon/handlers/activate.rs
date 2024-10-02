use crate::common::constants::TERRAINIUMD_TMP_DIR;
use crate::common::types::pb;
use crate::common::types::pb::{ActivateRequest, ActivateResponse};
use crate::daemon::handlers::execute::execute;
use crate::daemon::handlers::RequestHandler;
use crate::daemon::types::terrain_state::TerrainState;
use anyhow::{Context, Result};
use prost_types::Any;
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
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

fn get_state_dir_path(state: &TerrainState) -> String {
    format!(
        "{}/{}/{}",
        TERRAINIUMD_TMP_DIR,
        state.terrain_name(),
        state.timestamp()
    )
}

fn get_state_file_path(state: &TerrainState) -> String {
    format!("{}/state.json", get_state_dir_path(state))
}

async fn activate(request: ActivateRequest) -> Result<ActivateResponse> {
    let execute_request = request.clone().execute.expect("to have execute request");
    let state: TerrainState = request.into();

    let state_file = fs::File::options()
        .create(true)
        .truncate(true)
        .write(true)
        .open(get_state_file_path(&state))
        .await
        .expect("failed to create / append to log file");

    let state_file = Mutex::new(state_file);

    let mut guard = state_file.lock().await;

    guard
        .write_all(
            state
                .to_json()
                .expect("expected state to be serialized to json")
                .as_ref(),
        )
        .await
        .expect("Failed to write state to file");

    drop(guard);

    execute(execute_request, Some(Arc::new(state_file))).await;

    Ok(ActivateResponse {})
}
