use crate::common::types::pb;
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

    pub(crate) fn substitute_cwd(&mut self, terrain_dir: &Path) {
        // todo!()
    }

    pub fn example() -> Self {
        Command {
            exe: String::from("/bin/ls"),
            args: vec!["-a".to_string(), "-l".to_string()],
            cwd: None,
        }
    }
}

impl From<Command> for pb::Command {
    fn from(val: Command) -> Self {
        let cwd = match val.cwd.clone() {
            None => "".to_string(),
            Some(path) => path.display().to_string(),
        };

        pb::Command {
            exe: val.exe,
            args: val.args,
            envs: BTreeMap::default(),
            cwd,
        }
    }
}
