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
    pub should_fail_to_execute: bool,
    pub output: String,
}

pub struct AssertExecutor {
    executor: MockExecutor,
}

fn get_exit_status(exit_code: i32) -> ExitStatus {
    ExitStatus::from_raw(exit_code << 8)
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
            command,
            exit_code,
            should_fail_to_execute,
            ..
        } = command;

        self.executor
            .expect_wait()
            .with(eq(envs), eq(command), eq(silent))
            .returning(move |_, _, _| {
                if should_fail_to_execute {
                    bail!("failed to execute command");
                } else {
                    Ok(get_exit_status(exit_code))
                }
            })
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
            should_fail_to_execute,
            output,
        } = command;
        self.executor
            .expect_get_output()
            .with(eq(envs), eq(command))
            .returning(move |_, _| {
                if should_fail_to_execute {
                    bail!("failed to execute command");
                } else {
                    if exit_code != 0 {
                        Ok(Output {
                            // ExitStatus.code() returns status >> 8 so doing expected << 8
                            status: get_exit_status(exit_code),
                            stdout: vec![],
                            stderr: Vec::from(output.as_bytes()),
                        })
                    } else {
                        Ok(Output {
                            status: get_exit_status(exit_code),
                            stdout: Vec::from(output.as_bytes()),
                            stderr: vec![],
                        })
                    }
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
            should_fail_to_execute,
            ..
        } = command;

        self.executor
            .expect_async_spawn()
            .with(eq(envs), eq(command))
            .returning(move |_, _| {
                if should_fail_to_execute {
                    bail!("failed to execute command");
                } else {
                    Ok(get_exit_status(exit_code))
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
            should_fail_to_execute,
            ..
        } = command;

        self.executor
            .expect_async_spawn_with_log()
            .with(eq(log_path), eq(envs), eq(command))
            .returning(move |_, _, _| {
                if should_fail_to_execute {
                    bail!("failed to execute command");
                } else {
                    Ok(get_exit_status(exit_code))
                }
            })
            .times(times);
        self
    }

    pub fn successfully(self) -> MockExecutor {
        self.executor
    }
}
