use crate::common::types::pb;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct RunCommand {
    pub exe: &'static str,
    pub args: Vec<String>,
    pub env_vars: BTreeMap<String, String>,
    pub cwd: PathBuf,
    pub exit_code: i32,
    pub error: bool,
    pub output: &'static str,
    pub no_env: bool,
}

impl RunCommand {
    pub fn with_exe(exe: &'static str) -> Self {
        Self {
            exe,
            args: vec![],
            env_vars: Default::default(),
            cwd: Default::default(),
            exit_code: 0,
            error: false,
            output: "",
            no_env: false,
        }
    }

    pub fn with_arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }

    pub fn with_env(mut self, key: &'static str, val: &str) -> Self {
        self.env_vars.insert(key.to_string(), val.to_string());
        self
    }

    pub fn with_no_envs(mut self) -> Self {
        self.no_env = true;
        self
    }

    pub fn with_cwd(mut self, cwd: &Path) -> Self {
        self.cwd = cwd.to_path_buf();
        self
    }

    pub fn with_expected_exit_code(mut self, code: i32) -> Self {
        self.exit_code = code;
        self
    }

    pub fn with_expected_error(mut self, error: bool) -> Self {
        self.error = error;
        self
    }

    pub fn with_expected_output(mut self, output: &'static str) -> Self {
        self.output = output;
        self
    }
}

impl PartialEq<RunCommand> for pb::Command {
    fn eq(&self, other: &RunCommand) -> bool {
        self.exe == other.exe
            && self.args == other.args
            && self.envs == other.env_vars
            && self.cwd == other.cwd.to_str().unwrap()
    }
}
