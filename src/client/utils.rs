use crate::client::types::client::MockClient;
use crate::common::execute::MockCommandToRun;
use crate::common::types::pb;
use crate::common::types::pb::ExecuteRequest;
use prost_types::Any;
use std::cmp::PartialEq;
use std::collections::BTreeMap;
use std::os::unix::prelude::ExitStatusExt;
use std::process::{ExitStatus, Output};

#[derive(Clone)]
pub struct AssertExecuteRequest {
    terrain_name: String,
    biome_name: &'static str,
    toml_path: String,
    is_activate: bool,
    operation: &'static str,
    commands: Vec<RunCommand>,
    reply: Any,
}

impl AssertExecuteRequest {
    pub fn with() -> Self {
        Self {
            terrain_name: "".to_string(),
            biome_name: "",
            is_activate: false,
            operation: "",
            toml_path: "".to_string(),
            commands: vec![],
            reply: Default::default(),
        }
    }

    pub fn not_sent() -> MockClient {
        MockClient::default()
    }

    pub fn terrain_name(mut self, name: &str) -> Self {
        self.terrain_name = name.to_string();
        self
    }

    pub fn biome_name(mut self, name: &'static str) -> Self {
        self.biome_name = name;
        self
    }

    pub fn operation(mut self, name: &'static str) -> Self {
        self.operation = name;
        self
    }

    pub fn with_command(mut self, command: RunCommand) -> Self {
        self.commands.push(command);
        self
    }

    pub fn is_activated_as(mut self, is_active: bool) -> Self {
        self.is_activate = is_active;
        self
    }

    pub fn with_expected_reply(mut self, reply: Any) -> Self {
        self.reply = reply;
        self
    }

    pub fn toml_path(mut self, toml_path: &str) -> Self {
        self.toml_path = toml_path.to_string();
        self
    }

    pub fn sent(self) -> MockClient {
        let mut mock_client = MockClient::default();
        let this = self.clone();
        mock_client
            .expect_write_and_stop()
            .withf(move |execute_request| {
                let request: ExecuteRequest =
                    Any::to_msg(execute_request).expect("request to be converted");
                request.terrain_name == this.terrain_name
                    && request.biome_name == this.biome_name
                    && request.is_activate == this.is_activate
                    && request.toml_path == this.toml_path
                    && request.commands == this.commands
            })
            .return_once(move |_| Ok(()))
            .times(1);

        mock_client
            .expect_read()
            .with()
            .return_once(move || Ok(self.reply.clone()))
            .times(1);

        mock_client
    }
}

#[derive(Clone)]
pub struct RunCommand {
    exe: &'static str,
    args: Vec<&'static str>,
    env_vars: BTreeMap<String, String>,
    exit_code: i32,
    error: bool,
    output: &'static str,
}

impl RunCommand {
    pub fn with_exe(exe: &'static str) -> Self {
        Self {
            exe,
            args: vec![],
            env_vars: Default::default(),
            exit_code: 0,
            error: false,
            output: "",
        }
    }

    pub fn with_arg(mut self, arg: &'static str) -> Self {
        self.args.push(arg);
        self
    }

    pub fn with_env(mut self, key: &'static str, val: &str) -> Self {
        self.env_vars.insert(key.to_string(), val.to_string());
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
        self.exe == other.exe && self.args == other.args && self.envs == other.env_vars
    }
}

pub struct ExpectShell {
    runner: MockCommandToRun,
}

impl ExpectShell {
    pub fn to() -> Self {
        Self {
            runner: Default::default(),
        }
    }

    pub fn and(self) -> Self {
        self
    }

    pub fn successfully(self) -> MockCommandToRun {
        self.runner
    }

    pub fn get_output_of(mut self, command: RunCommand) -> Self {
        let mut mock_spawn = MockCommandToRun::default();
        mock_spawn
            .expect_set_args()
            .withf(move |args| *args == command.args.clone())
            .return_const(());

        mock_spawn
            .expect_set_envs()
            .withf(move |envs| *envs == Some(command.env_vars.clone()))
            .return_const(());

        mock_spawn.expect_get_output().with().return_once(move || {
            if command.error {
                Err(anyhow::Error::msg("error"))
            } else {
                Ok(Output {
                    status: ExitStatus::from_raw(command.exit_code),
                    stdout: Vec::from(command.output),
                    stderr: vec![],
                })
            }
        });

        self.runner
            .expect_clone()
            .times(1)
            .with()
            .return_once(|| mock_spawn);
        self
    }

    pub fn spawn_command(mut self, command: RunCommand) -> Self {
        let mut mock_spawn = MockCommandToRun::default();
        mock_spawn
            .expect_set_args()
            .withf(move |args| *args == command.args.clone())
            .return_const(());

        mock_spawn
            .expect_set_envs()
            .withf(move |envs| *envs == Some(command.env_vars.clone()))
            .return_const(());

        mock_spawn.expect_async_spawn().with().return_once(move || {
            if command.error {
                Err(anyhow::Error::msg("error"))
            } else {
                Ok(ExitStatus::from_raw(command.exit_code))
            }
        });

        self.runner
            .expect_clone()
            .times(1)
            .with()
            .return_once(|| mock_spawn);
        self
    }
}
