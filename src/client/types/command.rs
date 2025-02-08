use crate::client::types::biome::Biome;
use crate::client::validation::{
    Target, ValidationFixAction, ValidationMessageLevel, ValidationResult, ValidationResults,
};
use crate::common::types::pb;
use anyhow::{anyhow, Context, Result};
#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp::PartialEq;
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fmt::Display;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum CommandsType {
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
pub(crate) enum OperationType {
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
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Command {
    exe: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
}

impl Command {
    pub(crate) fn new(exe: String, args: Vec<String>, cwd: Option<PathBuf>) -> Self {
        Command { exe, args, cwd }
    }

    pub(crate) fn exe(&self) -> &str {
        &self.exe
    }

    pub(crate) fn args(&self) -> &[String] {
        &self.args
    }

    pub(crate) fn cwd(&self) -> Option<PathBuf> {
        self.cwd.clone()
    }

    pub(crate) fn substitute_cwd(
        &mut self,
        terrain_dir: &Path,
        envs: &BTreeMap<String, String>,
    ) -> Result<()> {
        if let Some(cwd) = &self.cwd {
            let cwd_str = cwd.to_str().unwrap();
            let envs_to_sub = Biome::get_envs_to_substitute(cwd_str);
            let cwd = if !envs_to_sub.is_empty() {
                &PathBuf::from(Biome::recursive_substitute_envs(
                    envs,
                    cwd_str.to_string(),
                    envs_to_sub,
                ))
            } else {
                cwd
            };
            if !cwd.is_absolute() {
                self.cwd = Some(terrain_dir.join(cwd).canonicalize().context(format!(
                    "failed to normalize path for exe:'{}' args: '{}' terrain_dir: '{terrain_dir:?}' and cwd: '{cwd:?}'",
                    self.exe, self.args.join(" ")
                ))?);
            }
        } else {
            self.cwd = Some(terrain_dir.to_path_buf());
        }
        Ok(())
    }

    pub(crate) fn validate_command<'a>(
        &'a self,
        biome_name: &'a str,
        operation_type: OperationType,
        commands_type: CommandsType,
        terrain_dir: &'a Path,
    ) -> ValidationResults<'a> {
        let mut result = HashSet::new();
        let exe = self.exe.as_str();

        if exe.is_empty() {
            result.insert(ValidationResult {
                level: ValidationMessageLevel::Error,
                message: format!(
                    "exe cannot be empty, make sure it is set before {commands_type} {operation_type} is to be run.",
                ),
                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                fix_action: ValidationFixAction::None,
            });
            return ValidationResults::new(result);
        }

        if exe.starts_with(" ") || exe.ends_with(" ") {
            result.insert(ValidationResult {
                level: ValidationMessageLevel::Warn,
                message: format!("exe '{}' has leading / trailing spaces. make sure it is removed before {commands_type} {operation_type} is to be run.", &self.exe),
                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                fix_action: ValidationFixAction::Trim { biome_name, target: Target::from_command(&commands_type, &operation_type, self) },
            });
        }
        let trimmed = exe.trim();

        if trimmed.contains(" ") {
            result.insert(ValidationResult {
                level: ValidationMessageLevel::Error,
                message: format!("exe '{}' contains whitespaces.", &self.exe,),
                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                fix_action: ValidationFixAction::None,
            });
        }

        let exe_path = PathBuf::from(trimmed);
        let executable_validation_message = ValidationResult {
            level: ValidationMessageLevel::Warn,
            message: format!("exe '{}' does not have permissions to execute. make sure it has correct permissions before {commands_type} {operation_type} is to be run.", exe),
            r#for: format!("{biome_name}({operation_type}:{commands_type})"),
            fix_action: ValidationFixAction::None,
        };

        if exe_path.is_absolute() {
            if !exe_path.exists() {
                result.insert(ValidationResult {
                    level: ValidationMessageLevel::Warn,
                    message: format!("exe '{:?}' does not exists. make sure it is present before {commands_type} {operation_type} is to be run.", trimmed),
                    r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                    fix_action: ValidationFixAction::None,
                });
            } else if !is_executable(&exe_path) {
                result.insert(executable_validation_message);
            }
        } else if trimmed.starts_with("./") || trimmed.starts_with("../") {
            let wd = self.cwd.clone().unwrap_or(terrain_dir.to_path_buf());
            let exe_path = wd.join(trimmed);
            if !exe_path.exists() {
                result.insert(ValidationResult {
                    level: ValidationMessageLevel::Warn,
                    message: format!("exe '{}' is not present in dir: {:?}. make sure it is present before {commands_type} {operation_type} is to be run.", trimmed, wd),
                    r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                    fix_action: ValidationFixAction::None,
                });
            } else if !is_executable(&exe_path) {
                result.insert(executable_validation_message);
            }
        } else {
            let res = is_exe_in_path(&self.exe);
            if let Some(exe_path) = res {
                if !is_executable(&exe_path) {
                    result.insert(executable_validation_message);
                }
            } else {
                result.insert(ValidationResult {
                    level: ValidationMessageLevel::Warn,
                    message: format!("exe '{}' is not present in PATH variable. make sure it is present before {commands_type} {operation_type} is to be run.", &self.exe),
                    r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                    fix_action: ValidationFixAction::None,
                });
            }
        }

        if trimmed.contains("sudo") {
            if commands_type == CommandsType::Background {
                result.insert(ValidationResult {
                    level: ValidationMessageLevel::Warn,
                    message: format!("command exe: '{trimmed}' args: '{}' uses sudo. Running sudo commands in background is not allowed (see terrainium docs for more info).", self.args.join(" ")),
                    r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                    fix_action: ValidationFixAction::None,
                });
            } else {
                result.insert(ValidationResult {
                    level: ValidationMessageLevel::Warn,
                    message: format!("command exe: '{trimmed}' args: '{}' uses sudo. Running sudo commands in foreground will block entering / exiting shell till user is authenticated.", self.args.join(" ")),
                    r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                    fix_action: ValidationFixAction::None,
                });
            }
        }

        if let Some(cwd) = self.cwd.clone() {
            let path_that_does_not_exits = if cwd.is_absolute() && !cwd.exists() {
                Some(cwd.clone())
            } else {
                let cwd = terrain_dir.join(cwd.clone());
                if !cwd.exists() {
                    Some(cwd)
                } else {
                    None
                }
            };

            if let Some(cwd) = path_that_does_not_exits {
                let envs_to_sub = Biome::get_envs_to_substitute(&cwd.display().to_string());
                if !envs_to_sub.is_empty() {
                    result.insert(ValidationResult {
                        level: ValidationMessageLevel::Info,
                        message: format!(
                            "cwd: '{}' contains environment variable references: '{}' for exe: '{trimmed}' args: '{}'. Make sure they are set before the {commands_type} {operation_type} is executed",
                            cwd.display(),
                            envs_to_sub.join("', '"),
                            self.args.join(" "),
                        ),
                        r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                        fix_action: ValidationFixAction::None,
                    });
                } else {
                    result.insert(ValidationResult {
                        level: ValidationMessageLevel::Warn,
                        message: format!(
                            "cwd: '{}' does not exists for command exe: '{trimmed}' args: '{}'.",
                            cwd.display(),
                            self.args.join(" ")
                        ),
                        r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                        fix_action: ValidationFixAction::None,
                    });
                }
            } else if (cwd.is_absolute() && !cwd.is_dir())
                || !terrain_dir.join(cwd.clone()).is_dir()
            {
                result.insert(ValidationResult {
                    level: ValidationMessageLevel::Warn,
                    message: format!(
                        "cwd: '{}' is not a directory for command exe: '{trimmed}' args: '{}'.",
                        cwd.display(),
                        self.args.join(" ")
                    ),
                    r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                    fix_action: ValidationFixAction::None,
                });
            }
        }

        ValidationResults::new(result)
    }
}

fn is_exe_in_path(exe: &str) -> Option<PathBuf> {
    if let Ok(path) = env::var("PATH") {
        for p in path.split(':') {
            let p_str = format!("{p}/{exe}");
            if fs::metadata(&p_str).is_ok() {
                return Some(PathBuf::from(p_str));
            }
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    let md = fs::metadata(path);
    if md.is_err() {
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
                "'cwd' is required for command exe:'{}', args: '{}'",
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
