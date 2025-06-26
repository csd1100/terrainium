use std::collections::{BTreeMap, HashSet};
use std::fmt::Display;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::client::types::biome::Biome;
use crate::client::validation::{
    Target, ValidationFixAction, ValidationMessageLevel, ValidationResult, ValidationResults,
};
use crate::common::types::pb;

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

fn is_exe_in_path(exe: &str) -> Option<PathBuf> {
    if let Ok(path) = std::env::var("PATH") {
        for p in path.split(':') {
            let p_str = format!("{p}/{exe}");
            if std::fs::metadata(&p_str).is_ok() {
                return Some(PathBuf::from(p_str));
            }
        }
    }
    None
}

fn resolve_symlink(path: &Path) -> PathBuf {
    if path.exists() && path.is_symlink() {
        let path = fs::read_link(path).unwrap();
        resolve_symlink(&path)
    } else {
        path.to_path_buf()
    }
}

fn is_executable(path: &Path) -> bool {
    let path = if path.is_symlink() {
        resolve_symlink(path)
    } else {
        path.to_path_buf()
    };

    if !path.is_file() {
        return false;
    }

    let md = std::fs::metadata(path);
    if md.is_err() {
        return false;
    }
    let permissions = md.unwrap().permissions();
    let mode = permissions.mode();

    mode & 0o111 != 0
}

#[cfg_attr(feature = "terrain-schema", derive(JsonSchema))]
#[derive(Debug, PartialEq, Clone, Hash, Eq, Serialize, Deserialize)]
pub struct Command {
    exe: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cwd = self
            .cwd
            .as_ref()
            .unwrap_or(&Default::default())
            .to_string_lossy()
            .to_string();

        let cwd = if cwd.is_empty() {
            String::from("terrain directory")
        } else {
            format!("'{cwd}'")
        };

        write!(f, "`{} {}` in {}", self.exe, self.args.join(" "), cwd)
    }
}

impl Command {
    pub fn new(exe: String, args: Vec<String>, cwd: Option<PathBuf>) -> Self {
        Command { exe, args, cwd }
    }

    pub fn trim_exe(mut self) -> Self {
        self.exe = self.exe.trim().to_string();
        self
    }

    pub fn exe(&self) -> &str {
        &self.exe
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn cwd(&self) -> &Option<PathBuf> {
        &self.cwd
    }

    pub fn set_args(&mut self, args: Vec<String>) {
        self.args = args;
    }

    pub fn set_cwd(&mut self, cwd: Option<PathBuf>) {
        self.cwd = cwd;
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
                    "failed to normalize path for exe:'{}' args: '{}' terrain_dir: \
                     '{terrain_dir:?}' and cwd: '{cwd:?}'",
                    self.exe,
                    self.args.join(" ")
                ))?);
            }
        } else {
            self.cwd = Some(terrain_dir.to_path_buf());
        }
        Ok(())
    }

    fn validate_path_exe(
        &self,
        exe_path: &Path,
        is_in_path: bool,
        biome_name: &str,
        operation_type: &OperationType,
        commands_type: &CommandsType,
        results: &mut HashSet<ValidationResult>,
    ) {
        let message = if !exe_path.exists() {
            if is_in_path {
                Some(format!(
                    "exe '{}' is not present in PATH variable. make sure it is present before \
                     {commands_type} {operation_type} is to be run.",
                    &self.exe
                ))
            } else {
                Some(format!(
                    "exe '{}' does not exists. make sure it is present before {commands_type} \
                     {operation_type} is to be run.",
                    self.exe
                ))
            }
        } else if !is_executable(exe_path) {
            Some(format!(
                "exe '{}' does not have permissions to execute. make sure it has correct \
                 permissions before {commands_type} {operation_type} is to be run.",
                self.exe
            ))
        } else {
            None
        };

        if let Some(message) = message {
            results.insert(ValidationResult {
                level: ValidationMessageLevel::Error,
                message,
                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                fix_action: ValidationFixAction::None,
            });
        }
    }

    fn validate_absent_cwd_for_envs(
        &self,
        operation_type: &OperationType,
        commands_type: &CommandsType,
        path: PathBuf,
    ) -> Option<(String, ValidationMessageLevel)> {
        let envs_to_sub = Biome::get_envs_to_substitute(path.to_str().unwrap());
        if !envs_to_sub.is_empty() {
            Some((
                format!(
                    "cwd: '{}' contains environment variable references: '{}' for exe: '{}' args: \
                     '{}'. Make sure they are set before the {commands_type} {operation_type} is \
                     executed",
                    path.display(),
                    envs_to_sub.join("', '"),
                    self.exe,
                    self.args.join(" "),
                ),
                ValidationMessageLevel::Info,
            ))
        } else {
            Some((
                format!(
                    "cwd: '{}' does not exists for command exe: '{}' args: '{}'.",
                    path.display(),
                    self.exe,
                    self.args.join(" ")
                ),
                ValidationMessageLevel::Error,
            ))
        }
    }

    fn validate_present_path(&self, path: PathBuf) -> Option<(String, ValidationMessageLevel)> {
        if path.is_symlink() {
            let resolved = fs::read_link(&path).unwrap();
            if !resolved.is_dir() {
                Some((
                    format!(
                        "cwd: '{}' is a symlink but does not resolve to directory ({}) for \
                         command exe: '{}' args: '{}'.",
                        path.display(),
                        resolved.display(),
                        self.exe,
                        self.args.join(" ")
                    ),
                    ValidationMessageLevel::Error,
                ))
            } else {
                None
            }
        } else if !path.is_dir() {
            Some((
                format!(
                    "cwd: '{}' is not a directory for command exe: '{}' args: '{}'.",
                    path.display(),
                    self.exe,
                    self.args.join(" ")
                ),
                ValidationMessageLevel::Error,
            ))
        } else {
            None
        }
    }

    fn validate_cwd(
        &self,
        operation_type: &OperationType,
        commands_type: &CommandsType,
        terrain_dir: &Path,
    ) -> Option<(String, ValidationMessageLevel)> {
        let cwd = self.cwd.clone().unwrap();
        let (cwd, exists) = if cwd.is_absolute() {
            let exists = cwd.exists();
            (cwd, exists)
        } else {
            let cwd = terrain_dir.join(cwd);
            let exists = cwd.exists();
            (cwd, exists)
        };

        if !exists {
            self.validate_absent_cwd_for_envs(operation_type, commands_type, cwd)
        } else {
            self.validate_present_path(cwd)
        }
    }

    pub(crate) fn validate_command<'a>(
        &'a self,
        biome_name: &'a str,
        operation_type: &'a OperationType,
        commands_type: &'a CommandsType,
        terrain_dir: &'a Path,
    ) -> ValidationResults<'a> {
        let mut results = HashSet::new();
        let mut fixable = false;
        let exe = self.exe.as_str();

        if exe.is_empty() {
            results.insert(ValidationResult {
                level: ValidationMessageLevel::Error,
                message: format!(
                    "exe cannot be empty, make sure it is set before {commands_type} \
                     {operation_type} is to be run.",
                ),
                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                fix_action: ValidationFixAction::None,
            });
            return ValidationResults::new(false, results);
        }

        if exe.starts_with(" ") || exe.ends_with(" ") {
            fixable = true;
            results.insert(ValidationResult {
                level: ValidationMessageLevel::Warn,
                message: format!(
                    "exe '{}' has leading / trailing spaces. make sure it is removed before \
                     {commands_type} {operation_type} is to be run.",
                    &self.exe
                ),
                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                fix_action: ValidationFixAction::Trim {
                    biome_name,
                    target: Target::from_command(commands_type, operation_type, self),
                },
            });
        }

        let trimmed = exe.trim();
        if trimmed.contains(" ") {
            results.insert(ValidationResult {
                level: ValidationMessageLevel::Error,
                message: format!("exe '{}' contains whitespaces.", &self.exe),
                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                fix_action: ValidationFixAction::None,
            });
        }

        if trimmed.contains("sudo") {
            let message = match commands_type {
                CommandsType::Foreground => {
                    format!(
                        "command exe: '{trimmed}' args: '{}' uses sudo. Running sudo commands in \
                         foreground will block entering / exiting shell till user is \
                         authenticated.",
                        self.args.join(" ")
                    )
                }
                CommandsType::Background => {
                    format!(
                        "command exe: '{trimmed}' args: '{}' uses sudo. Running sudo commands in \
                         background is not allowed (see terrainium docs for more info).",
                        self.args.join(" ")
                    )
                }
            };

            results.insert(ValidationResult {
                level: ValidationMessageLevel::Warn,
                message,
                r#for: format!("{biome_name}({operation_type}:{commands_type})"),
                fix_action: ValidationFixAction::None,
            });
        }

        let exe_path = PathBuf::from(trimmed);

        if exe_path.is_absolute() {
            self.validate_path_exe(
                &exe_path,
                false,
                biome_name,
                operation_type,
                commands_type,
                &mut results,
            );
        } else if trimmed.starts_with("./") || trimmed.starts_with("../") {
            let wd = self.cwd.as_ref().map_or(terrain_dir, |wd| wd);
            let exe_path = wd.join(trimmed);

            self.validate_path_exe(
                &exe_path,
                false,
                biome_name,
                operation_type,
                commands_type,
                &mut results,
            );
        } else {
            let res = is_exe_in_path(trimmed).unwrap_or_default();
            self.validate_path_exe(
                &res,
                true,
                biome_name,
                operation_type,
                commands_type,
                &mut results,
            );
        }

        if self.cwd.is_none() {
            return ValidationResults::new(fixable, results);
        }

        let message_and_level = self.validate_cwd(operation_type, commands_type, terrain_dir);

        if message_and_level.is_none() {
            return ValidationResults::new(fixable, results);
        }

        let (message, level) = message_and_level.unwrap();
        results.insert(ValidationResult {
            level,
            message,
            r#for: format!("{biome_name}({operation_type}:{commands_type})"),
            fix_action: ValidationFixAction::None,
        });

        ValidationResults::new(fixable, results)
    }
}

impl From<Command> for std::process::Command {
    fn from(value: Command) -> std::process::Command {
        let mut command = std::process::Command::new(value.exe);
        command
            .args(value.args)
            .current_dir(value.cwd.expect("cwd to be present"));
        command
    }
}

impl From<Command> for tokio::process::Command {
    fn from(value: Command) -> tokio::process::Command {
        let mut command = tokio::process::Command::new(value.exe);
        command
            .args(value.args)
            .current_dir(value.cwd.expect("cwd to be present"));
        command
    }
}

impl From<Command> for pb::Command {
    fn from(value: Command) -> Self {
        let Command { exe, args, cwd, .. } = value;
        Self {
            exe,
            args,
            cwd: cwd.unwrap().to_string_lossy().to_string(),
        }
    }
}

impl From<pb::Command> for Command {
    fn from(value: pb::Command) -> Self {
        Self {
            exe: value.exe,
            args: value.args,
            cwd: Some(PathBuf::from(value.cwd)),
        }
    }
}
