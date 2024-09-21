use crate::common::types::command::Command;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct Commands {
    foreground: Vec<Command>,
    background: Vec<Command>,
}

impl Commands {
    pub fn new(foreground: Vec<Command>, background: Vec<Command>) -> Self {
        Commands {
            foreground,
            background,
        }
    }

    pub fn example() -> Self {
        Commands {
            foreground: vec![Command::example()],
            background: vec![Command::example()],
        }
    }
}
