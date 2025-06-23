use std::path::PathBuf;

#[derive(Debug, Default, Clone)]
pub struct DaemonPaths {
    dir: PathBuf,
}

impl DaemonPaths {
    pub fn new(dir: &str) -> DaemonPaths {
        DaemonPaths {
            dir: PathBuf::from(dir),
        }
    }

    pub fn dir(&self) -> &PathBuf {
        &self.dir
    }

    pub fn dir_str(&self) -> &str {
        self.dir.to_str().unwrap()
    }

    pub fn socket(&self) -> PathBuf {
        self.dir.join("socket")
    }

    pub fn pid(&self) -> PathBuf {
        self.dir.join("pid")
    }
}

pub fn get_terrainiumd_paths() -> DaemonPaths {
    let dir = if cfg!(debug_assertions) {
        "/tmp/terrainiumd-debug"
    } else {
        "/tmp/terrainiumd"
    };
    DaemonPaths::new(dir)
}
