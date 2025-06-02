use crate::common::types::pb;
use crate::common::types::pb::response::Payload::{Body, Error};
use crate::common::types::pb::{Activate, Response};
use crate::daemon::handlers::RequestHandler;
use crate::daemon::types::context::DaemonContext;
use crate::daemon::types::terrain_state::TerrainState;
use anyhow::{Context, Result};
use prost_types::Any;
use tracing::{error, trace, warn};

pub(crate) struct ActivateHandler;

impl RequestHandler for ActivateHandler {
    async fn handle(request: Any, context: DaemonContext) -> Any {
        trace!("handling Activate request");
        let activate: Result<Activate> = request
            .to_msg()
            .context("failed to convert request to Activate");

        trace!("result of attempting to parse request: {:#?}", activate);

        let response = match activate {
            Ok(activate) => create_state(activate, context).await,
            Err(err) => Response {
                payload: Some(Error(err.to_string())),
            },
        };
        Any::from_msg(&response).unwrap()
    }
}

async fn create_state(request: Activate, context: DaemonContext) -> Response {
    trace!("creating state for {request:#?}");
    let state: TerrainState = request.into();
    let result = context.state_manager().create_state(state).await;
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
