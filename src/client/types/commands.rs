use crate::client::types::command::Command;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

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

    pub(crate) fn substitute_cwd(&mut self, terrain_dir: &Path) -> Result<()> {
        self.foreground
            .iter_mut()
            .try_for_each(|command| command.substitute_cwd(terrain_dir))?;
        self.background
            .iter_mut()
            .try_for_each(|command| command.substitute_cwd(terrain_dir))
    }

    pub fn example() -> Self {
        Commands {
            foreground: vec![Command::example()],
            background: vec![Command::example()],
        }
    }
}
