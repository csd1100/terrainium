use std::{io::Read, os::unix::net::UnixStream};

use anyhow::{Context, Result};
use prost::{bytes::Bytes, Message};

use crate::proto::Command;

pub fn handle(mut stream: UnixStream) -> Result<()> {
    let mut data = vec![];
    stream.read_to_end(&mut data)?;
    let _data: Command = Command::decode(Bytes::from(data)).context("unable to parse command")?;
    Ok(())
}
