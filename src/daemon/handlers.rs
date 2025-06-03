use crate::common::types::pb::response::Payload::Error;
use crate::common::types::pb::Response;
use crate::common::types::socket::Socket;
use crate::daemon::handlers::activate::ActivateHandler;
use crate::daemon::handlers::deactivate::DeactivateHandler;
use crate::daemon::handlers::execute::ExecuteHandler;
use crate::daemon::types::context::DaemonContext;
#[mockall_double::double]
use crate::daemon::types::daemon_socket::DaemonSocket;
use anyhow::{Context, Result};
use prost_types::Any;
use tracing::{error, event, instrument, Level};

mod activate;
mod deactivate;
mod execute;

pub(crate) trait RequestHandler {
    async fn handle(request: Any, context: DaemonContext) -> Any;
}

#[instrument(skip(daemon_socket))]
pub async fn handle_request(mut daemon_socket: DaemonSocket, context: DaemonContext) {
    event!(Level::INFO, "handling requests on socket");

    let data: Result<Any> = daemon_socket
        .read()
        .await
        .context("failed to read daemon socket");

    let response = match data {
        Ok(request) => match request.type_url.as_str() {
            "/terrainium.v1.Activate" => ActivateHandler::handle(request, context).await,
            "/terrainium.v1.Execute" => ExecuteHandler::handle(request, context).await,
            "/terrainium.v1.Deactivate" => DeactivateHandler::handle(request, context).await,
            "/terrainium.v1.StatusRequest" => {
                todo!()
            }
            _ => {
                event!(Level::ERROR, "invalid request type: {:?}", request.type_url);

                Any::from_msg(&Response {
                    payload: Some(Error(format!(
                        "invalid request type {:?}",
                        request.type_url
                    ))),
                })
                .expect("failed to create an error response")
            }
        },
        Err(err) => Any::from_msg(&error_response(err)).expect("failed to create an error"),
    };

    let result = daemon_socket.write_and_stop(response).await;

    if result.is_err() {
        event!(
            Level::ERROR,
            "error responding execute request: {:?}",
            result
        );
    }
}

pub fn error_response(err: anyhow::Error) -> Response {
    let error = format!("{:?}", err);
    error!("{}", error);
    Response {
        payload: Some(Error(error)),
    }
}
