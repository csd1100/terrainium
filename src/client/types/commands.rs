use crate::client::types::command::{Command, CommandsType, OperationType};
use crate::client::validation::ValidationResults;
use anyhow::{Context, Result};
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

    pub(crate) fn substitute_cwd(&mut self, terrain_dir: &Path) -> Result<()> {
        self.foreground
            .iter_mut()
            .try_for_each(|command| command.substitute_cwd(terrain_dir))
            .context("failed to substitute cwd for foreground commands")?;

        self.background
            .iter_mut()
            .try_for_each(|command| command.substitute_cwd(terrain_dir))
            .context("failed to substitute cwd for background commands")
    }

    #[cfg(test)]
    pub fn foreground_mut(&mut self) -> &mut Vec<Command> {
        self.foreground.as_mut()
    }

    #[cfg(test)]
    pub fn background_mut(&mut self) -> &mut Vec<Command> {
        self.background.as_mut()
    }

    pub(crate) fn validate_commands(
        &self,
        biome_name: &str,
        operation_type: OperationType,
        terrain_dir: &Path,
    ) -> ValidationResults {
        let mut result = ValidationResults::new(vec![]);

        self.foreground.iter().for_each(|c| {
            result.append(&mut c.validate_command(
                biome_name,
                &operation_type,
                CommandsType::Foreground,
                terrain_dir,
            ))
        });

        self.background.iter().for_each(|c| {
            result.append(&mut c.validate_command(
                biome_name,
                &operation_type,
                CommandsType::Background,
                terrain_dir,
            ))
        });

        result
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
