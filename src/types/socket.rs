use std::{
    io::{Read, Write},
    net::Shutdown,
    ops::{Deref, DerefMut},
    os::unix::net::UnixStream,
};

use anyhow::Result;
#[cfg(test)]
use mockall::automock;
use prost::{bytes::Bytes, Message};

use crate::helpers::constants::TERRAINIUMD_SOCK;

pub struct Unix {
    stream: UnixStream,
}

#[cfg_attr(test, automock)]
impl Unix {
    pub fn new() -> Result<Self> {
        let stream = UnixStream::connect(TERRAINIUMD_SOCK)?;
        let socket = Unix { stream };
        Ok(socket)
    }

    pub fn write<T: Message + 'static>(&mut self, data: T) -> Result<()> {
        let stream = self.deref_mut();
        stream.write_all(&data.encode_to_vec())?;
        stream.shutdown(Shutdown::Write)?;
        Ok(())
    }

    pub fn read(&mut self) -> Result<Bytes> {
        let mut data = vec![];
        self.read_to_end(&mut data)?;
        Ok(Bytes::from(data))
    }
}

impl Drop for Unix {
    fn drop(&mut self) {
        let _ = self.deref().shutdown(Shutdown::Both);
    }
}

impl Deref for Unix {
    type Target = UnixStream;

    fn deref(&self) -> &Self::Target {
        &self.stream
    }
}

impl DerefMut for Unix {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.stream
    }
}

impl From<UnixStream> for Unix {
    fn from(value: UnixStream) -> Self {
        Self { stream: value }
    }
}
