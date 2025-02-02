use crate::common::types::pb;
use anyhow::{anyhow, Context, Result};
use home::home_dir;
#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[cfg_attr(feature = "terrain-schema", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Command {
    exe: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
}
impl Command {
    pub fn new(exe: String, args: Vec<String>, cwd: Option<PathBuf>) -> Self {
        Command { exe, args, cwd }
    }

    pub(crate) fn substitute_cwd(&mut self, terrain_dir: &Path) -> Result<()> {
        if let Some(cwd) = &self.cwd {
            if !cwd.is_absolute() {
                self.cwd = Some(terrain_dir.join(cwd).canonicalize().context(format!(
                    "failed to normalize path for exe:'{}' args: '{}' terrain_dir: '{:?}' and cwd: '{:?}'",
                    self.exe, self.args.join(" "), terrain_dir, cwd
                ))?);
            }
        } else {
            self.cwd = Some(terrain_dir.to_path_buf());
        }
        Ok(())
    }

    pub fn example() -> Self {
        Command {
            exe: String::from("/bin/ls"),
            args: vec!["-a".to_string(), "-l".to_string()],
            cwd: home_dir(),
        }
    }
}

impl TryFrom<Command> for pb::Command {
    type Error = anyhow::Error;

    fn try_from(value: Command) -> Result<Self> {
        if value.cwd.is_none() {
            return Err(anyhow!(
                "`cwd` is required for command exe:`{}`, args: `{}`",
                value.exe,
                value.args.join(" ")
            ));
        }
        Ok(pb::Command {
            exe: value.exe,
            args: value.args,
            envs: BTreeMap::default(),
            cwd: value.cwd.unwrap().to_string_lossy().to_string(),
        })
    }
}
