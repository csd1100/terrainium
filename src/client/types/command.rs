use crate::common::types::pb;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;

#[cfg_attr(feature = "terrain-schema", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Command {
    exe: String,
    args: Vec<String>,
}
impl Command {
    pub fn new(exe: String, args: Vec<String>) -> Self {
        Command { exe, args }
    }

    pub fn example() -> Self {
        Command {
            exe: String::from("/bin/ls"),
            args: vec!["-a".to_string(), "-l".to_string()],
        }
    }
}

impl From<Command> for pb::Command {
    fn from(val: Command) -> Self {
        pb::Command {
            exe: val.exe,
            args: val.args,
            envs: BTreeMap::default(),
        }
    }
}
