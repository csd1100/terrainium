use serde::{Deserialize, Serialize};

use crate::handlers::helpers::get_merged_vecs;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Commands {
    pub exec: Vec<Command>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Command {
    pub exe: String,
    pub args: Option<Vec<String>>,
}

impl Commands {
    pub fn merge(&self, other: Self) -> Self {
        let execs = get_merged_vecs(&self.exec, &other.exec);
        return Commands { exec: execs };
    }
}
