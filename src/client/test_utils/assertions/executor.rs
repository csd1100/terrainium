use crate::common::execute::MockExecutor;
use crate::common::types::command::Command;
use anyhow::bail;
use mockall::predicate::eq;
use std::collections::BTreeMap;
use std::os::unix::prelude::ExitStatusExt;
use std::process::{ExitStatus, Output};
use std::sync::Arc;

#[derive(Clone)]
pub struct ExpectedCommand {
    pub command: Command,
    pub exit_code: i32,
    pub should_error: bool,
    pub output: String,
}

pub struct AssertExecutor {
    executor: MockExecutor,
}

impl AssertExecutor {
    pub fn to() -> Self {
        Self {
            executor: MockExecutor::default(),
        }
    }

    pub fn with(executor: MockExecutor) -> Self {
        Self { executor }
    }

    pub fn wait_for(
        mut self,
        envs: Option<Arc<BTreeMap<String, String>>>,
        command: ExpectedCommand,
        silent: bool,
        times: usize,
    ) -> MockExecutor {
        let ExpectedCommand {
            command, exit_code, ..
        } = command;

        self.executor
            .expect_wait()
            .with(eq(envs), eq(command), eq(silent))
            .returning(move |_, _, _| Ok(ExitStatus::from_raw(exit_code)))
            .times(times);

        self.executor
    }

    pub fn get_output_for(
        mut self,
        envs: Option<Arc<BTreeMap<String, String>>>,
        command: ExpectedCommand,
        times: usize,
    ) -> Self {
        let ExpectedCommand {
            command,
            exit_code,
            should_error: error,
            output,
        } = command;
        self.executor
            .expect_get_output()
            .with(eq(envs), eq(command))
            .returning(move |_, _| {
                if error {
                    Ok(Output {
                        // ExitStatus.code() returns status >> 8 so doing expected << 8
                        status: ExitStatus::from_raw(exit_code << 8),
                        stdout: vec![],
                        stderr: Vec::from(output.as_bytes()),
                    })
                } else {
                    Ok(Output {
                        status: ExitStatus::from_raw(0),
                        stdout: Vec::from(output.as_bytes()),
                        stderr: vec![],
                    })
                }
            })
            .times(times);
        self
    }

    pub fn async_spawn(
        mut self,
        envs: Option<Arc<BTreeMap<String, String>>>,
        command: ExpectedCommand,
        times: usize,
    ) -> Self {
        let ExpectedCommand {
            command,
            exit_code,
            should_error,
            output,
        } = command;

        self.executor
            .expect_async_spawn()
            .with(eq(envs), eq(command))
            .returning(move |_, _| {
                if should_error {
                    bail!("{}", output)
                } else {
                    Ok(ExitStatus::from_raw(exit_code << 8))
                }
            })
            .times(times);
        self
    }

    pub fn async_spawn_with_log(
        mut self,
        command: ExpectedCommand,
        envs: Option<Arc<BTreeMap<String, String>>>,
        log_path: String,
        times: usize,
    ) -> Self {
        let ExpectedCommand {
            command,
            exit_code,
            should_error,
            output,
        } = command;

        self.executor
            .expect_async_spawn_with_log()
            .with(eq(log_path), eq(envs), eq(command))
            .returning(move |_, _, _| {
                if should_error {
                    bail!("{}", output)
                } else {
                    Ok(ExitStatus::from_raw(exit_code << 8))
                }
            })
            .times(times);
        self
    }

    pub fn successfully(self) -> MockExecutor {
        self.executor
    }
}
