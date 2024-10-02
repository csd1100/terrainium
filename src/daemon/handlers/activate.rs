use crate::common::types::pb;
use crate::common::types::pb::{
    ActivateRequest, ActivateResponse, ExecuteRequest, ExecuteResponse,
};
use crate::daemon::handlers::RequestHandler;
use anyhow::Context;
use prost_types::Any;
use tracing::{event, Level};

pub(crate) struct ActivateHandler;

impl RequestHandler for ActivateHandler {
    async fn handle(request: Any) -> Any {
        event!(Level::INFO, "handling ActivateRequest");
        let act_request: anyhow::Result<ActivateRequest> = request
            .to_msg()
            .context("failed to convert request to type ActivateRequest");

        event!(
            Level::DEBUG,
            "result of attempting to parse request: {:#?}",
            act_request
        );

        match act_request {
            Ok(request) => {
                let response: ActivateResponse = activate(request).await;
                Any::from_msg(&response).expect("to be converted to Any")
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

async fn activate(request: ActivateRequest) {}
