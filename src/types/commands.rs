use serde::{Deserialize, Serialize};

use crate::handlers::helpers::get_merged_vecs;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Commands {
    pub foreground: Option<Vec<Command>>,
    pub background: Option<Vec<Command>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Command {
    pub exe: String,
    pub args: Option<Vec<String>>,
}

impl Commands {
    pub fn merge(&self, other: Self) -> Self {
        let foreground = get_merged_vecs(&self.foreground, &other.foreground);
        let background = get_merged_vecs(&self.background, &other.background);
        return Commands {
            foreground,
            background,
        };
    }
}
