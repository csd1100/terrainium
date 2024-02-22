use anyhow::{anyhow, Context, Result};
use prost::Message;

use crate::{
    proto::{command::CommandType, Command},
    types::socket,
};

pub fn handle(mut socket: socket::Unix) -> Result<()> {
    let data: Command = Command::decode(socket.read()?).context("unable to parse command")?;
    match CommandType::try_from(data.r#type)? {
        CommandType::Unspecified => return Err(anyhow!("unspecified command found")),
        CommandType::Execute => println!("Execute"),
        CommandType::Status => println!("Status: {:?}", data),
    }
    Ok(())
}
