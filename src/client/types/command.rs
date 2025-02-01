use crate::client::validation::{ValidationMessageLevel, ValidationResult, ValidationResults};
use crate::common::types::pb;
use anyhow::{anyhow, Context, Result};
use home::home_dir;
#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};
use std::{env, result};

#[derive(Clone, Debug)]
pub enum CommandsType {
    Foreground,
    Background,
}

impl Display for CommandsType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandsType::Foreground => {
                write!(f, "foreground")
            }
            CommandsType::Background => {
                write!(f, "background")
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum OperationType {
    Constructor,
    Destructor,
}

impl Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationType::Constructor => {
                write!(f, "constructor")
            }
            OperationType::Destructor => {
                write!(f, "destructor")
            }
        }
    }
}

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

    pub(crate) fn validate_command(
        &self,
        biome_name: &str,
        operation_type: &OperationType,
        commands_type: CommandsType,
    ) -> ValidationResults {
        let mut result = vec![];

        if self.exe.starts_with(" ") || self.exe.ends_with(" ") {
            result.push(ValidationResult {
                level: ValidationMessageLevel::Warn,
                message: format!("exe `{}` has leading / trailing spaces. make sure it is removed {} {} is to be run.", &self.exe, commands_type, operation_type),
                target: format!("{}({}:{})", biome_name, operation_type, commands_type),
            });
        }
        let trimmed = self.exe.trim();

        if trimmed.contains(" ") {
            result.push(ValidationResult {
                level: ValidationMessageLevel::Error,
                message: format!("exe `{}` contains whitespaces.", &self.exe,),
                target: format!("{}({}:{})", biome_name, operation_type, commands_type),
            });
        }

        if !is_exe_in_path(&self.exe) {
            result.push(ValidationResult {
                level: ValidationMessageLevel::Warn,
                message: format!("exe `{}` is not present in PATH variable. make sure it is present before {} {} is to be run.", &self.exe, commands_type, operation_type),
                target: format!("{}({}:{})", biome_name, operation_type, commands_type),
            });
        }

        ValidationResults::new(result)
    }
}

fn is_exe_in_path(exe: &str) -> bool {
    if let Ok(path) = env::var("PATH") {
        for p in path.split(':') {
            let p_str = format!("{}/{}", p, exe);
            if fs::metadata(p_str).is_ok() {
                return true;
            }
        }
    }
    false
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
