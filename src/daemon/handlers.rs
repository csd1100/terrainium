use crate::common::types::pb;
use crate::common::types::socket::Socket;
use crate::daemon::handlers::execute::ExecuteHandler;
use crate::daemon::handlers::status::StatusHandler;
use crate::daemon::handlers::status_poll::StatusPollHandler;
#[mockall_double::double]
use crate::daemon::types::daemon_socket::DaemonSocket;
use anyhow::Result;
use prost_types::Any;
use tracing::{event, instrument, Level};

pub mod execute;
pub mod status;
pub mod status_poll;

pub(crate) trait RequestHandler {
    async fn handle(request: Any) -> Any;
}

#[instrument(skip(daemon_socket))]
pub async fn handle_request(mut daemon_socket: DaemonSocket) {
    event!(Level::INFO, "handling requests on socket");

    let data: Result<Any> = daemon_socket.read().await;
    event!(Level::TRACE, "data received on socket: {:?} ", data);

    let response = match data {
        Ok(request) => match request.type_url.as_str() {
            "/terrainium.v1.ExecuteRequest" => ExecuteHandler::handle(request).await,

            "/terrainium.v1.StatusRequest" => StatusHandler::handle(request).await,

            "/terrainium.v1.StatusPoll" => StatusPollHandler::handle(request).await,

            _ => {
                event!(Level::ERROR, "invalid request type: {:?}", request.type_url);

                Any::from_msg(&pb::Error {
                    error_message: format!("invalid request type {:?}", request.type_url),
                })
                .expect("failed to create an error response")
            }
        },
        Err(err) => {
            event!(Level::ERROR, "failed to read data from socket: {:?}", err);

            Any::from_msg(&pb::Error {
                error_message: err.to_string(),
            })
            .expect("failed to create an error response")
        }
    };

    event!(Level::TRACE, "sending response to client: {:?}", response);
    let result = daemon_socket.write_and_stop(response).await;

    if result.is_err() {
        event!(
            Level::ERROR,
            "error responding execute request: {:?}",
            result
        );
    }
}
