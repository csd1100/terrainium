use crate::common::types::pb;
use crate::common::types::socket::Socket;
use crate::daemon::handlers::run::ExecuteHandler;
#[double]
use crate::daemon::types::daemon_socket::DaemonSocket;
use anyhow::Result;
use mockall_double::double;
use prost_types::Any;
use tracing::{event, instrument, Level};

pub mod run;

pub(crate) trait RequestHandler {
    async fn handle(request: Any) -> Any;
}

#[instrument(skip(daemon_socket))]
pub async fn handle_request(mut daemon_socket: DaemonSocket) {
    event!(Level::INFO, "handling requests on socket");

    let data: Result<Any> = daemon_socket.read().await;
    event!(Level::DEBUG, "data received on socket: {:?} ", data);

    let response = match data {
        Ok(request) => match request.type_url.as_str() {
            "/terrainium.v1.ExecuteRequest" => ExecuteHandler::handle(request).await,

            "/terrainium.v1.StatusRequest" => {
                todo!()
            }
            _ => {
                event!(Level::ERROR, "invalid request type: {:?}", request.type_url);

                Any::from_msg(&pb::Error {
                    error_message: format!("invalid request type {:?}", request.type_url),
                })
                .expect("failed to create an error response")
            }
        },
        Err(err) => {
            event!(Level::ERROR, "failed to read data from socket: {:#?}", err);

            Any::from_msg(&pb::Error {
                error_message: err.to_string(),
            })
            .expect("failed to create an error response")
        }
    };

    event!(Level::INFO, "sending response to client: {:#?}", response);
    let result = daemon_socket.write_and_stop(response).await;

    if result.is_err() {
        event!(
            Level::ERROR,
            "error responding execute request: {:?}",
            result
        );
    }
}
