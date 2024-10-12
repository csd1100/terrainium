use crate::common::types::pb;
use crate::common::types::pb::StatusResponse;
use crate::common::types::terrain_state::TerrainState;
use crate::daemon::handlers::RequestHandler;
use crate::daemon::types::history::get_from_history;
use anyhow::{Context, Result};
use prost_types::Any;
use tokio::fs;
use tracing::{event, instrument, Level};

pub(crate) struct StatusHandler;

impl RequestHandler for StatusHandler {
    #[instrument(skip(request))]
    async fn handle(request: Any) -> Any {
        event!(Level::INFO, "handling ExecuteRequest");

        let status_request: Result<pb::StatusRequest> = request
            .to_msg()
            .context("failed to convert to StatusRequest");

        event!(
            Level::DEBUG,
            "result of attempting to parse request: {:?}",
            status_request
        );

        match status_request {
            Ok(status_request) => {
                let status = get_status(status_request)
                    .await
                    .context("failed to get status");

                status.unwrap_or_else(|err| {
                    event!(Level::ERROR, "failed to create the response {:?}", err);
                    Any::from_msg(&pb::Error {
                        error_message: err.to_string(),
                    })
                    .expect("to be converted to Any")
                })
            }
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
async fn get_status(request: pb::StatusRequest) -> Result<Any> {
    let state_path = get_from_history(request).context("failed to get status from request")?;
    event!(Level::INFO, "getting state from {:?}", &state_path);
    let json = fs::read_to_string(&state_path)
        .await
        .unwrap_or_else(|_| panic!("failed to read state file: {:?}", &state_path));

    let state = TerrainState::from_json(&json).expect("failed to parse state json");
    event!(
        Level::DEBUG,
        "fetched state from: {:?}, state: {:?}",
        state_path,
        state
    );

    let response: StatusResponse = state.into();

    Ok(Any::from_msg(&response).expect("to be converted to Any"))
}
