use crate::common::types::pb;
use crate::common::types::pb::response::Payload::Body;
use crate::common::types::pb::{Response, StatusRequest};
use crate::daemon::handlers::{error_response, RequestHandler};
use crate::daemon::types::context::DaemonContext;
use anyhow::{bail, Context, Result};
use prost_types::Any;
use std::sync::Arc;
use tracing::trace;

pub struct StatusHandler;

impl RequestHandler for StatusHandler {
    async fn handle(request: Any, context: Arc<DaemonContext>) -> Any {
        trace!("handling Status request");
        let request: Result<StatusRequest> = request
            .to_msg()
            .context("failed to convert request to Activate");

        let response = match request {
            Ok(data) => status(data, context)
                .await
                .context("failed to handle status request")
                .unwrap_or_else(error_response),
            Err(err) => error_response(err),
        };
        Any::from_msg(&response).unwrap()
    }
}

async fn status(request: StatusRequest, context: Arc<DaemonContext>) -> Result<Response> {
    let StatusRequest {
        identifier,
        terrain_name,
    } = request;

    if identifier.is_none() {
        bail!("identifier missing from status request");
    }

    let stored_history = context
        .state_manager()
        .get_or_create_history(&terrain_name)
        .await
        .context(format!("failed to create history file {terrain_name}"))?;

    let session_id = stored_history
        .read()
        .await
        .get_session(identifier.unwrap())
        .context("failed to get session id from history")?;

    let stored_state = context
        .state_manager()
        .refreshed_state(&terrain_name, &session_id)
        .await
        .context("failed to fetch the state")?;

    let state: pb::StatusResponse = stored_state.read().await.state().into();
    trace!(
        terrain_name = terrain_name,
        session_id = session_id,
        "successfully fetched state {state:#?}"
    );
    Ok(Response {
        payload: Some(Body(pb::Body {
            message: Some(state),
        })),
    })
}
