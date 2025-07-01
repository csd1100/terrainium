use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::pb;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Command {
    exe: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
}

impl Command {
    /// creates a new command object for Executor to use
    pub fn new(exe: String, args: Vec<String>, cwd: Option<PathBuf>) -> Self {
        Command { exe, args, cwd }
    }

    /// Get executable for the command
    pub fn exe(&self) -> &str {
        &self.exe
    }

    /// Get the arguments for the command
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Get the path to the directory where [Command] is to be run
    pub fn cwd(&self) -> &Option<PathBuf> {
        &self.cwd
    }

    /// set arguments for command to be executed
    pub fn set_args(&mut self, args: Vec<String>) {
        self.args = args;
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

impl From<Command> for std::process::Command {
    fn from(value: Command) -> std::process::Command {
        let mut command = std::process::Command::new(value.exe);
        command
            .args(value.args)
            .current_dir(value.cwd.expect("cwd to be present"));
        command
    }
}
