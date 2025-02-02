use crate::client::validation::{ValidationMessageLevel, ValidationResult, ValidationResults};
use crate::common::types::pb;
use anyhow::{anyhow, Context, Result};
use home::home_dir;
#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fmt::Display;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

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
        terrain_dir: &Path,
    ) -> ValidationResults {
        let mut result = vec![];
        let exe = self.exe.as_str();

        if exe.starts_with(" ") || exe.ends_with(" ") {
            result.push(ValidationResult {
                level: ValidationMessageLevel::Warn,
                message: format!("exe `{}` has leading / trailing spaces. make sure it is removed {} {} is to be run.", &self.exe, commands_type, operation_type),
                target: format!("{}({}:{})", biome_name, operation_type, commands_type),
            });
        }
        let trimmed = exe.trim();

        if trimmed.contains(" ") {
            result.push(ValidationResult {
                level: ValidationMessageLevel::Error,
                message: format!("exe `{}` contains whitespaces.", &self.exe,),
                target: format!("{}({}:{})", biome_name, operation_type, commands_type),
            });
        }

        let exe_path = PathBuf::from(trimmed);
        let executable_validation_message = ValidationResult {
            level: ValidationMessageLevel::Warn,
            message: format!("exe `{}` does not have permissions to execute. make sure it has correct permissions before {} {} is to be run.", exe, commands_type, operation_type),
            target: format!("{}({}:{})", biome_name, operation_type, commands_type),
        };

        if exe_path.is_absolute() {
            if !exe_path.exists() {
                result.push(ValidationResult {
                    level: ValidationMessageLevel::Warn,
                    message: format!("exe `{:?}` does not exists. make sure it is present before {} {} is to be run.", trimmed, commands_type, operation_type),
                    target: format!("{}({}:{})", biome_name, operation_type, commands_type),
                });
            } else if !is_executable(&exe_path) {
                result.push(executable_validation_message);
            }
        } else if trimmed.starts_with("./") || trimmed.starts_with("../") {
            let wd = self.cwd.clone().unwrap_or(terrain_dir.to_path_buf());
            let exe_path = wd.join(trimmed);
            if !exe_path.exists() {
                result.push(ValidationResult {
                    level: ValidationMessageLevel::Warn,
                    message: format!("exe `{}` is not present in dir: {:?}. make sure it is present before {} {} is to be run.", trimmed, wd, commands_type, operation_type),
                    target: format!("{}({}:{})", biome_name, operation_type, commands_type),
                });
            } else if !is_executable(&exe_path) {
                result.push(executable_validation_message);
            }
        } else {
            let res = is_exe_in_path(&self.exe);
            if let Some(exe_path) = res {
                if !is_executable(&exe_path) {
                    result.push(executable_validation_message);
                }
            } else {
                result.push(ValidationResult {
                level: ValidationMessageLevel::Warn,
                message: format!("exe `{}` is not present in PATH variable. make sure it is present before {} {} is to be run.", &self.exe, commands_type, operation_type),
                target: format!("{}({}:{})", biome_name, operation_type, commands_type),
            });
            }
        }

        ValidationResults::new(result)
    }
}

fn is_exe_in_path(exe: &str) -> Option<PathBuf> {
    if let Ok(path) = env::var("PATH") {
        for p in path.split(':') {
            let p_str = format!("{}/{}", p, exe);
            if fs::metadata(&p_str).is_ok() {
                return Some(PathBuf::from(p_str));
            }
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    let md = fs::metadata(path);
    if let Err(err) = md {
        return false;
    }
    let permissions = md.unwrap().permissions();
    let mode = permissions.mode();

    mode & 0o111 != 0
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
