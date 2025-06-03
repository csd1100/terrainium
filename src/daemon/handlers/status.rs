use crate::common::types::pb;
use crate::common::types::pb::response::Payload::Body;
use crate::common::types::pb::{Response, StatusRequest};
use crate::daemon::handlers::{error_response, RequestHandler};
use crate::daemon::types::context::DaemonContext;
use anyhow::Context;
use prost_types::Any;
use tracing::trace;

pub struct StatusHandler;

impl RequestHandler for StatusHandler {
    async fn handle(request: Any, context: DaemonContext) -> Any {
        trace!("handling Status request");
        let request: anyhow::Result<StatusRequest> = request
            .to_msg()
            .context("failed to convert request to Activate");

        trace!("result of attempting to parse request: {:#?}", request);

        let response = match request {
            Ok(data) => status(data, context).await,
            Err(err) => error_response(err),
        };
        Any::from_msg(&response).unwrap()
    }
}

async fn status(request: StatusRequest, context: DaemonContext) -> Response {
    let StatusRequest {
        session_id,
        terrain_name,
    } = request;
    let result = context
        .state_manager()
        .refreshed_state(&terrain_name, &session_id)
        .await;

    match result {
        Ok(stored_state) => {
            let state = stored_state.read().await;
            let state: pb::StatusResponse = state.state().into();
            Response {
                payload: Some(Body(pb::Body {
                    message: Some(state),
                })),
            }
        }
        Err(err) => error_response(err),
    }
}
