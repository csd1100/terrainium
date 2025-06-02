use crate::common::types::pb;
use crate::common::types::pb::response::Payload::{Body, Error};
use crate::common::types::pb::{Activate, Response};
use crate::daemon::handlers::construct::spawn_constructors;
use crate::daemon::handlers::RequestHandler;
use crate::daemon::types::context::DaemonContext;
use crate::daemon::types::terrain_state::TerrainState;
use anyhow::{Context, Result};
use prost_types::Any;
use tracing::{error, trace};

pub(crate) struct ActivateHandler;

impl RequestHandler for ActivateHandler {
    async fn handle(request: Any, context: DaemonContext) -> Any {
        trace!("handling Activate request");
        let request: Result<Activate> = request
            .to_msg()
            .context("failed to convert request to Activate");

        trace!("result of attempting to parse request: {:#?}", request);

        let response = match request {
            Ok(data) => activate(data, context).await,
            Err(err) => Response {
                payload: Some(Error(err.to_string())),
            },
        };
        Any::from_msg(&response).unwrap()
    }
}

async fn activate(request: Activate, context: DaemonContext) -> Response {
    let terrain_name = request.terrain_name.clone();
    let session_id = request.session_id.clone();
    trace!("activating terrain {terrain_name}({session_id})");

    let constructors = request.constructors.clone();
    let mut result = create_state(request, &context).await;
    if result.is_ok() {
        trace!("successfully created state for terrain {terrain_name}({session_id})");
        if let Some(constructors) = constructors {
            trace!("spawning constructors for terrain {terrain_name}({session_id})");
            result = spawn_constructors(constructors, context).await;
        }
    }
    match result {
        Ok(()) => Response {
            payload: Some(Body(pb::Body { message: None })),
        },
        Err(err) => {
            error!("failed to create state due to an error {err:#?}");
            Response {
                payload: Some(Error(err.to_string())),
            }
        }
    }
}

async fn create_state(request: Activate, context: &DaemonContext) -> Result<()> {
    trace!("creating state for {request:#?}");
    let state: TerrainState = request.into();
    context.state_manager().create_state(&state).await
}
