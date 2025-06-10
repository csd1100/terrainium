use crate::common::types::pb;
use crate::common::types::pb::response::Payload::Body;
use crate::common::types::pb::{Activate, Response};
use crate::common::types::terrain_state::TerrainState;
use crate::daemon::handlers::execute::spawn_commands;
use crate::daemon::handlers::{error_response, RequestHandler};
use crate::daemon::types::context::DaemonContext;
use anyhow::{Context, Result};
use prost_types::Any;
use std::sync::Arc;
use tracing::trace;

pub(crate) struct ActivateHandler;

impl RequestHandler for ActivateHandler {
    async fn handle(request: Any, context: Arc<DaemonContext>) -> Any {
        trace!("handling Activate request");
        let request: Result<Activate> = request
            .to_msg()
            .context("failed to convert request to Activate");

        let response = match request {
            Ok(data) => activate(data, context)
                .await
                .context("failed to activate")
                .unwrap_or_else(error_response),
            Err(err) => error_response(err),
        };
        Any::from_msg(&response).unwrap()
    }
}

async fn activate(request: Activate, context: Arc<DaemonContext>) -> Result<Response> {
    let terrain_name = request.terrain_name.clone();
    let session_id = request.session_id.clone();
    trace!(
        terrain_name = terrain_name,
        session_id = session_id,
        "starting activation of terrain"
    );

    let constructors = request.constructors.clone();
    create_state(request, &context)
        .await
        .context("failed to create state while activating")?;

    trace!(
        terrain_name = terrain_name,
        session_id = session_id,
        "successfully created state"
    );
    if let Some(constructors) = constructors {
        trace!(
            terrain_name = terrain_name,
            session_id = session_id,
            "spawning constructors for activation request"
        );
        spawn_commands(constructors, context)
            .await
            .context("failed to spawn constructors while activating")?;
    }

    Ok(Response {
        payload: Some(Body(pb::Body { message: None })),
    })
}

async fn create_state(request: Activate, context: &DaemonContext) -> Result<()> {
    trace!("creating state for {request:#?}");
    let state: TerrainState = request.into();
    context.state_manager().create_state(state).await?;
    Ok(())
}
