use crate::common::types::command::Command;
use serde::{Deserialize, Serialize};

#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;

#[cfg_attr(feature = "terrain-schema", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Commands {
    foreground: Vec<Command>,
    background: Vec<Command>,
}

impl Commands {
    pub fn foreground(&self) -> &Vec<Command> {
        self.foreground.as_ref()
    }

    pub fn background(&self) -> &Vec<Command> {
        self.background.as_ref()
    }

    pub(crate) fn append(&mut self, another: &mut Commands) {
        self.foreground.append(&mut another.foreground);
        self.background.append(&mut another.background);
    }
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
