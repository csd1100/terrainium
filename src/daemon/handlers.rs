use crate::common::types::pb;
use crate::common::types::pb::response::Payload::Error;
use crate::common::types::pb::Response;
use crate::common::types::socket::Socket;
use crate::daemon::handlers::activate::ActivateHandler;
use crate::daemon::handlers::construct::ConstructHandler;
use crate::daemon::types::context::DaemonContext;
#[mockall_double::double]
use crate::daemon::types::daemon_socket::DaemonSocket;
use anyhow::Result;
use prost_types::Any;
use tracing::{error, event, instrument, Level};

mod activate;
mod construct;

pub(crate) trait RequestHandler {
    async fn handle(request: Any, context: DaemonContext) -> Any;
}

#[instrument(skip(daemon_socket))]
pub async fn handle_request(mut daemon_socket: DaemonSocket, context: DaemonContext) {
    event!(Level::INFO, "handling requests on socket");

    let data: Result<Any> = daemon_socket.read().await;
    // event!(Level::DEBUG, "data received on socket: {:?} ", data);

    let response = match data {
        Ok(request) => match request.type_url.as_str() {
            "/terrainium.v1.Activate" => ActivateHandler::handle(request, context).await,
            "/terrainium.v1.Construct" => ConstructHandler::handle(request, context).await,
            "/terrainium.v1.StatusRequest" => {
                todo!()
            }
            _ => {
                event!(Level::ERROR, "invalid request type: {:?}", request.type_url);

                Any::from_msg(&pb::Response {
                    payload: Some(Error(format!(
                        "invalid request type {:?}",
                        request.type_url
                    ))),
                })
                .expect("failed to create an error response")
            }
        },
        Err(err) => {
            event!(Level::ERROR, "failed to read data from socket: {:#?}", err);

            Any::from_msg(&pb::Response {
                payload: Some(Error(err.to_string())),
            })
            .expect("failed to create an error response")
        }
    };

    // event!(Level::TRACE, "sending response to client: {:#?}", response);
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
