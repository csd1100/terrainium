use crate::common::types::pb;
use crate::common::types::pb::response::Payload::Body;
use crate::common::types::pb::{Deactivate, Response};
use crate::daemon::handlers::execute::spawn_commands;
use crate::daemon::handlers::{error_response, RequestHandler};
use crate::daemon::types::context::DaemonContext;
use anyhow::{Context, Result};
use prost_types::Any;
use std::sync::Arc;
use tracing::trace;

pub struct DeactivateHandler;
impl RequestHandler for DeactivateHandler {
    async fn handle(request: Any, context: Arc<DaemonContext>) -> Any {
        trace!("handling deactivate request");
        let request: Result<Deactivate> = request
            .to_msg()
            .context("failed to convert request to Deactivate");

        let response = match request {
            Ok(data) => deactivate(data, context).await,
            Err(err) => error_response(err),
        };
        Any::from_msg(&response).unwrap()
    }
}

async fn deactivate(request: Deactivate, context: Arc<DaemonContext>) -> Response {
    let Deactivate {
        session_id,
        terrain_name,
        end_timestamp,
        destructors,
    } = request;
    trace!(
        terrain_name = terrain_name,
        session_id = session_id,
        end_timestamp = end_timestamp,
        "executing deactivate request"
    );
    let mut result = context
        .state_manager()
        .update_end_time(&terrain_name, &session_id, end_timestamp)
        .await
        .context("failed to deactivate");

    if result.is_ok() {
        trace!(
            terrain_name = terrain_name,
            session_id = session_id,
            "updated end time successfully"
        );
        if let Some(destructors) = destructors {
            trace!("running destructors for deactivation request");
            result = spawn_commands(destructors, context).await;
        }
    }

    match result {
        Ok(()) => Response {
            payload: Some(Body(pb::Body { message: None })),
        },
        Err(err) => error_response(err),
    }
}
