use crate::common::execute::MockExecutor;
use crate::common::types::command::Command;
use anyhow::bail;
use mockall::predicate::eq;
use std::os::unix::prelude::ExitStatusExt;
use std::process::{ExitStatus, Output};

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

    pub fn wait_for(mut self, command: ExpectedCommand) -> MockExecutor {
        let ExpectedCommand {
            command, exit_code, ..
        } = command;

        self.executor
            .expect_wait()
            .with(eq(command))
            .return_once(move |_| Ok(ExitStatus::from_raw(exit_code)));

        self.executor
    }

    pub fn get_output_for(mut self, command: ExpectedCommand) -> Self {
        let ExpectedCommand {
            command,
            exit_code,
            should_error: error,
            output,
        } = command;
        self.executor
            .expect_get_output()
            .with(eq(command))
            .return_once(move |_| {
                if error {
                    Ok(Output {
                        status: ExitStatus::from_raw(exit_code),
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
            });
        self
    }

    pub fn async_spawn(mut self, command: ExpectedCommand) -> Self {
        let ExpectedCommand {
            command,
            exit_code,
            should_error,
            output,
        } = command;

        self.executor
            .expect_async_spawn()
            .with(eq(command))
            .return_once(move |_| {
                if should_error {
                    bail!("{}", output)
                } else {
                    let ec = ExitStatus::from_raw(exit_code);
                    let some = ec.code();
                    println!("{:?}", some);
                    Ok(ec)
                }
            });
        self
    }

    pub fn successfully(self) -> MockExecutor {
        self.executor
    }
}
