use crate::common::constants::TERRAINIUMD_TMP_DIR;
use crate::common::types::pb;
use crate::common::types::pb::StatusPollResponse;
use crate::common::utils::string_to_time;
use crate::daemon::handlers::RequestHandler;
use anyhow::Context;
use prost_types::Any;
use tokio::fs;
use tracing::{event, Level};

pub struct StatusPollHandler;

impl RequestHandler for StatusPollHandler {
    async fn handle(request: Any) -> Any {
        event!(Level::INFO, "handling StatusPoll");

        let status_request: anyhow::Result<pb::StatusPoll> =
            request.to_msg().context("failed to convert to StatusPoll");

        event!(
            Level::DEBUG,
            "result of attempting to parse request: {:?}",
            status_request
        );

        match status_request {
            Ok(poll_request) => {
                let state_path = format!(
                    "{}/{}/{}/state.json",
                    TERRAINIUMD_TMP_DIR, poll_request.terrain_name, poll_request.session_id
                );

                let last_modified = fs::metadata(&state_path)
                    .await
                    .expect("failed to read metadata of state file")
                    .modified()
                    .expect("failed to get last modified time");

                if last_modified > string_to_time(&poll_request.last_modified) {
                    Any::from_msg(&StatusPollResponse { is_updated: true })
                        .expect("to be converted to Any")
                } else {
                    Any::from_msg(&StatusPollResponse { is_updated: false })
                        .expect("to be converted to Any")
                }
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
