use crate::client::validation::ValidationResults;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use crate::common::types::command::{Command, CommandsType, OperationType};
use crate::common::types::pb;
#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;

#[cfg_attr(feature = "terrain-schema", derive(JsonSchema))]
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Commands {
    foreground: Vec<Command>,
    background: Vec<Command>,
}

impl Commands {
    pub(crate) fn foreground(&self) -> &Vec<Command> {
        self.foreground.as_ref()
    }

    pub(crate) fn background(&self) -> &Vec<Command> {
        self.background.as_ref()
    }

    pub(crate) fn append(&mut self, another: &mut Commands) {
        self.foreground.append(&mut another.foreground);
        self.background.append(&mut another.background);
    }

    pub(crate) fn substitute_cwd(
        &mut self,
        terrain_dir: &Path,
        envs: &BTreeMap<String, String>,
    ) -> Result<()> {
        self.foreground
            .iter_mut()
            .try_for_each(|command| command.substitute_cwd(terrain_dir, envs))
            .context("failed to substitute cwd for foreground commands")?;

        self.background
            .iter_mut()
            .try_for_each(|command| command.substitute_cwd(terrain_dir, envs))
            .context("failed to substitute cwd for background commands")
    }

    pub(crate) fn foreground_mut(&mut self) -> &mut Vec<Command> {
        self.foreground.as_mut()
    }

    pub(crate) fn background_mut(&mut self) -> &mut Vec<Command> {
        self.background.as_mut()
    }

    pub(crate) fn to_proto_commands(&self) -> Result<Vec<pb::Command>> {
        self.background()
            .iter()
            .map(|c| Ok(c.clone().into()))
            .collect()
    }

    pub(crate) fn validate_commands<'a>(
        &'a self,
        biome_name: &'a str,
        operation_type: &'a OperationType,
        terrain_dir: &'a Path,
    ) -> ValidationResults<'a> {
        let mut result = ValidationResults::new(false, HashSet::new());

        self.foreground.iter().for_each(|c| {
            result.append(c.validate_command(
                biome_name,
                operation_type,
                &CommandsType::Foreground,
                terrain_dir,
            ))
        });

        self.background.iter().for_each(|c| {
            result.append(c.validate_command(
                biome_name,
                operation_type,
                &CommandsType::Background,
                terrain_dir,
            ))
        });

        result
    }
}

impl Commands {
    pub(crate) fn new(foreground: Vec<Command>, background: Vec<Command>) -> Self {
        Commands {
            foreground,
            background,
        }
    }
}
