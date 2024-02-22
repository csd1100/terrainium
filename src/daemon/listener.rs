use std::{
    os::unix::net::UnixListener,
    path::{Path, PathBuf},
};

use anyhow::Result;

pub struct Listener {
    path: PathBuf,
    listener: UnixListener,
}

impl Listener {
    pub fn bind(path: impl AsRef<Path>) -> Result<Self> {
        std::fs::remove_file(&path)?;
        let path = path.as_ref().to_owned();
        Ok(UnixListener::bind(&path).map(|listener| Self { path, listener })?)
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        std::fs::remove_file(&self.path).unwrap();
    }
}

impl std::ops::Deref for Listener {
    type Target = UnixListener;

    fn deref(&self) -> &Self::Target {
        &self.listener
    }
}

impl std::ops::DerefMut for Listener {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.listener
    }
}
