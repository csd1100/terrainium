use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use prost_types::Any;
use tracing::{debug, error, trace};

use crate::common::types::pb::Response;
use crate::common::types::pb::response::Payload::Error;
use crate::common::types::socket::Socket;
use crate::daemon::handlers::activate::ActivateHandler;
use crate::daemon::handlers::deactivate::DeactivateHandler;
use crate::daemon::handlers::execute::ExecuteHandler;
use crate::daemon::handlers::status::StatusHandler;
use crate::daemon::types::context::DaemonContext;
#[mockall_double::double]
use crate::daemon::types::daemon_socket::DaemonSocket;

mod activate;
mod deactivate;
mod execute;
mod status;

pub(crate) trait RequestHandler {
    async fn handle(request: Any, context: Arc<DaemonContext>) -> Any;
}

pub async fn handle_request(context: Arc<DaemonContext>, mut daemon_socket: DaemonSocket) {
    trace!("handling requests on socket");

    let data: Result<Any> = daemon_socket
        .read()
        .await
        .context("failed to read daemon socket");

    let response = match data {
        Ok(request) => {
            debug!("handling request of type {}", request.type_url);
            match request.type_url.as_str() {
                "/terrainium.v1.Activate" => ActivateHandler::handle(request, context).await,
                "/terrainium.v1.Execute" => ExecuteHandler::handle(request, context).await,
                "/terrainium.v1.Deactivate" => DeactivateHandler::handle(request, context).await,
                "/terrainium.v1.StatusRequest" => StatusHandler::handle(request, context).await,
                _ => {
                    let err = anyhow!("invalid request type: {:?}", request.type_url);
                    Any::from_msg(&error_response(err)).expect("failed to create an error response")
                }
            }
        }
        Err(err) => Any::from_msg(&error_response(err)).expect("failed to create an error"),
    };

    let result = daemon_socket.write_and_stop(response).await;

    if let Err(err) = result {
        error!("error responding to the request: {err:#?}");
    }
}

pub fn error_response(err: anyhow::Error) -> Response {
    let error = format!("{err:?}");
    error!("{}", error);
    Response {
        payload: Some(Error(error)),
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_handle_request() {}
}
