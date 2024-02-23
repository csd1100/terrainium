use anyhow::{anyhow, Context, Result};
use prost::Message;

use crate::{
    proto::{self},
    types::socket,
};

use super::handlers::activate;

fn session_id_from_history(history: i32) -> Result<String> {
    match proto::request::History::try_from(history)? {
        proto::request::History::Unspecified => {
            Err(anyhow!("invalid session history parameter found"))
        }
        proto::request::History::Last => Ok("last".to_string()),
        proto::request::History::Last1 => Ok("last1".to_string()),
        proto::request::History::Last2 => Ok("last2".to_string()),
    }
}

fn session_id_from(request: &proto::Request) -> Result<String> {
    match &request.session {
        Some(session) => match session {
            proto::request::Session::SessionId(session_id) => Ok(session_id.clone()),
            proto::request::Session::History(history) => session_id_from_history(*history),
        },
        None => Err(anyhow!("no session was found in request")),
    }
}

pub fn handle(mut socket: socket::Unix) -> Result<()> {
    let request: proto::Request =
        proto::Request::decode(socket.read()?).context("unable to parse request")?;

    let session_id = session_id_from(&request)?;

    let args = request.args;
    let result: Result<Option<proto::response::success::Body>, anyhow::Error> = args
        .map(|args| match args {
            proto::request::Args::Activate(request) => {
                activate::handle(session_id, request).map(proto::response::success::Body::Activate)
            }
            proto::request::Args::Execute(_) => todo!(),
            proto::request::Args::Status(_) => todo!(),
        })
        .transpose();

    let response: proto::Response = match result {
        Ok(data) => match data {
            Some(body) => proto::Response {
                result: Some(proto::response::Result::Success(proto::response::Success {
                    body: Some(body),
                })),
            },
            None => proto::Response {
                result: Some(proto::response::Result::Error(proto::response::Error {
                    error_message: "no response body was generated".to_string(),
                })),
            },
        },
        Err(err) => proto::Response {
            result: Some(proto::response::Result::Error(proto::response::Error {
                error_message: format!("error while generating response: {}", err),
            })),
        },
    };

    socket
        .write(response)
        .context("error writing response to client")?;

    Ok(())
}
