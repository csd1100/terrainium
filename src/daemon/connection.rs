use anyhow::{Context, Result};
use prost::Message;
use prost_types::Any;

use crate::{
    proto::{response, Command, Response},
    types::socket,
};

use super::handlers::activate;

pub fn handle(mut socket: socket::Unix) -> Result<()> {
    let data: Command = Command::decode(socket.read()?).context("unable to parse command")?;
    let args = data.args;
    let result = args
        .map(|args| match args {
            crate::proto::command::Args::Activate(request) => {
                activate::handle(request).and_then(|data| Ok(Any::from_msg(&data)?))
            }
            crate::proto::command::Args::Execute(_) => todo!(),
            crate::proto::command::Args::Status(_) => todo!(),
        })
        .transpose();

    let response = match result {
        Ok(data) => match data {
            Some(data) => Response {
                result: Some(response::Result::Success(data)),
            },
            None => Response {
                result: Some(response::Result::Error(crate::proto::Error {
                    error_message: "no response was generated".to_string(),
                })),
            },
        },
        Err(err) => Response {
            result: Some(response::Result::Error(crate::proto::Error {
                error_message: err.to_string(),
            })),
        },
    };

    socket
        .write(response)
        .context("error writing response to client")?;

    Ok(())
}
