use std::path::PathBuf;

use crate::constants::{TERRAINIUMD_TMP_DIR_DBG, TERRAINIUMD_TMP_DIR_REL};

/// Object that provides way to access all the paths used by Daemon
pub struct DaemonPaths {
    base: PathBuf,
}

impl DaemonPaths {
    /// Constructs the object
    pub fn get() -> Self {
        let base = if cfg!(debug_assertions) {
            TERRAINIUMD_TMP_DIR_DBG
        } else {
            TERRAINIUMD_TMP_DIR_REL
        };
        Self {
            base: PathBuf::from(base),
        }
    }

    /// Get Base directory
    ///
    /// For Release Build: base directory will be `/tmp/terrainiumd`
    /// For Debug Build: base directory will be `/tmp/terrainiumd-debug`
    pub fn base(&self) -> &PathBuf {
        &self.base
    }

    /// Get Base directory
    ///
    /// For Release Build: base directory will be `/tmp/terrainiumd`
    /// For Debug Build: base directory will be `/tmp/terrainiumd-debug`
    pub fn base_str(&self) -> &str {
        self.base.to_str().unwrap()
    }

    /// Get Socket File Path
    ///
    /// For Release Build: base directory will be `/tmp/terrainiumd/socket`
    /// For Debug Build: base directory will be `/tmp/terrainiumd-debug/socket`
    pub fn socket(&self) -> PathBuf {
        self.base.join("socket")
    }

    /// Get PID File Path
    ///
    /// For Release Build: base directory will be `/tmp/terrainiumd/pid`
    /// For Debug Build: base directory will be `/tmp/terrainiumd-debug/pid`
    pub fn pid(&self) -> PathBuf {
        self.base.join("pid")
    }

    #[cfg(any(test, feature = "test-exports"))]
    pub fn build(base: &str) -> Self {
        Self {
            base: PathBuf::from(base),
        }
    }
}
