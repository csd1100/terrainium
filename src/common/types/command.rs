use crate::client::types::biome::Biome;
use crate::client::validation::{
    Target, ValidationFixAction, ValidationMessageLevel, ValidationResult, ValidationResults,
};
use crate::common::constants::JSON;
use crate::common::types::pb;
use anyhow::{Context, Result};
#[cfg(feature = "terrain-schema")]
use schemars::JsonSchema;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::{BTreeMap, HashSet};
use std::fmt::Display;
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

fn is_executable(path: &Path) -> bool {
    let md = std::fs::metadata(path);
    if md.is_err() {
        return false;
    }
    let permissions = md.unwrap().permissions();
    let mode = permissions.mode();

    mode & 0o111 != 0
}

#[cfg_attr(feature = "terrain-schema", derive(JsonSchema))]
#[derive(Debug, PartialEq, Clone, Hash, Eq, Deserialize)]
pub struct Command {
    exe: String,
    args: Vec<String>,
    // do not serialize envs for terrain.toml
    #[cfg_attr(feature = "terrain-schema", schemars(skip))]
    #[serde(skip_serializing)]
    envs: Option<BTreeMap<String, String>>,
    cwd: Option<PathBuf>,
}

impl Serialize for Command {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let type_name = std::any::type_name::<S>();
        let is_json = type_name.contains(JSON);

        let mut command = serializer.serialize_struct("Command", if is_json { 4 } else { 3 })?;
        command.serialize_field("exe", &self.exe)?;
        command.serialize_field("args", &self.args)?;
        // do serialize envs for command state but do not serialize for toml
        // as for terrain.toml we have common env vars specified
        // only serialize if envs are set
        if is_json && self.envs.is_some() {
            command.serialize_field("envs", &self.envs)?;
        }
        command.serialize_field("cwd", &self.cwd)?;
        command.end()
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cwd = self
            .cwd
            .clone()
            .unwrap_or_default()
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
    pub fn new(
        exe: String,
        args: Vec<String>,
        envs: Option<BTreeMap<String, String>>,
        cwd: Option<PathBuf>,
    ) -> Self {
        Command {
            exe,
            args,
            envs,
            cwd,
        }
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

    pub fn set_envs(&mut self, envs: Option<BTreeMap<String, String>>) {
        self.envs = envs;
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
        let mut fixable = false;
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
            return ValidationResults::new(false, result);
        }

        if exe.starts_with(" ") || exe.ends_with(" ") {
            fixable = true;
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
                message: format!("exe '{}' contains whitespaces.", &self.exe),
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

        ValidationResults::new(fixable, result)
    }
}

impl From<Command> for std::process::Command {
    fn from(value: Command) -> std::process::Command {
        let mut vars: BTreeMap<String, String> = std::env::vars().collect();
        let envs = if let Some(mut envs) = value.envs {
            vars.append(&mut envs);
            vars
        } else {
            vars
        };
        let mut command = std::process::Command::new(value.exe);
        command
            .args(value.args)
            .envs(envs)
            .current_dir(value.cwd.expect("cwd to be present"));
        command
    }
}

impl From<Command> for tokio::process::Command {
    fn from(value: Command) -> tokio::process::Command {
        let mut vars: BTreeMap<String, String> = std::env::vars().collect();
        let envs = if let Some(mut envs) = value.envs {
            vars.append(&mut envs);
            vars
        } else {
            vars
        };
        let mut command = tokio::process::Command::new(value.exe);
        command
            .args(value.args)
            .envs(envs)
            .current_dir(value.cwd.expect("cwd to be present"));
        command
    }
}

impl From<Command> for pb::Command {
    fn from(value: Command) -> Self {
        let Command {
            exe,
            args,
            envs,
            cwd,
        } = value;
        Self {
            exe,
            args,
            envs: envs.unwrap_or_default(),
            cwd: cwd.unwrap().to_string_lossy().to_string(),
        }
    }
}

impl From<pb::Command> for Command {
    fn from(value: pb::Command) -> Self {
        Self {
            exe: value.exe,
            args: value.args,
            envs: Some(value.envs),
            cwd: Some(PathBuf::from(value.cwd)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn does_not_serialize_envs_to_toml() {
        let mut envs = BTreeMap::<String, String>::new();
        envs.insert("TEST".to_string(), "value".to_string());

        let command = Command::new("test".to_string(), vec![], Some(envs), None);

        assert_eq!(
            "exe = \"test\"\nargs = []\n",
            toml::to_string(&command).unwrap()
        );
    }

    #[test]
    fn does_serialize_envs_to_json() {
        let mut envs = BTreeMap::<String, String>::new();
        envs.insert("TEST".to_string(), "value".to_string());

        let command = Command::new("test".to_string(), vec![], Some(envs), None);

        assert_eq!(
            r#"{"exe":"test","args":[],"envs":{"TEST":"value"},"cwd":null}"#,
            serde_json::to_string(&command).unwrap()
        );
    }
}
