use crate::common::constants::TERRAINIUMD_TMP_DIR;
use crate::common::types::pb;
use crate::common::types::pb::StatusResponse;
use crate::common::types::terrain_state::TerrainState;
use crate::common::utils::time_to_string;
use crate::daemon::handlers::RequestHandler;
use anyhow::{Context, Result};
use prost_types::Any;
use tokio::fs;
use tracing::{event, instrument, Level};

pub(crate) struct StatusHandler;

impl RequestHandler for StatusHandler {
    #[instrument(skip(request))]
    async fn handle(request: Any) -> Any {
        event!(Level::INFO, "handling StatusRequest");

        let status_request: Result<pb::StatusRequest> = request
            .to_msg()
            .context("failed to convert to StatusRequest");

        event!(
            Level::DEBUG,
            "result of attempting to parse request: {:?}",
            status_request
        );

        match status_request {
            Ok(status_request) => get_status(status_request).await,
            Err(err) => {
                event!(Level::ERROR, "failed to parse the request {:?}", err);
                Any::from_msg(&pb::Error {
                    error_message: err.to_string(),
                })
                .expect("to be converted to Any")
            }
        }
    }
}

#[instrument(skip(request))]
async fn get_status(request: pb::StatusRequest) -> Any {
    let state_path = format!(
        "{}/{}/{}/state.json",
        TERRAINIUMD_TMP_DIR, request.terrain_name, request.session_id
    );

    event!(Level::INFO, "getting state from {}", state_path);
    let json = fs::read_to_string(&state_path)
        .await
        .unwrap_or_else(|_| panic!("failed to read state file: {}", state_path));

    let state = TerrainState::from_json(&json).expect("failed to parse state json");
    event!(
        Level::INFO,
        "fetched state from: {}, state: {:?}",
        state_path,
        state
    );

    let mut response: StatusResponse = state.into();

    let last_modified = fs::metadata(&state_path)
        .await
        .expect("failed to read metadata of state file")
        .modified()
        .expect("failed to get last modified time");

    response.last_modified = time_to_string(last_modified);

    Any::from_msg(&response).expect("to be converted to Any")
}
