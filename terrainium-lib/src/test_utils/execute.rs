use std::collections::BTreeMap;
use std::os::unix::process::ExitStatusExt;
use std::process::{ExitStatus, Output};
use std::sync::Arc;

use anyhow::bail;
use mockall::predicate::eq;

use crate::command::Command;
use crate::executor::MockExecute;

pub struct ExpectExecutor {
    executor: MockExecute,
}

fn get_exit_status(exit_code: i32) -> ExitStatus {
    ExitStatus::from_raw(exit_code << 8)
}

impl ExpectExecutor {
    pub fn to() -> Self {
        Self {
            executor: MockExecute::new(),
        }
    }

    pub fn with(executor: MockExecute) -> Self {
        Self { executor }
    }

    /// run command successfully and return 0 exit code
    pub fn successfully_get_output_for(
        self,
        envs: Option<Arc<BTreeMap<String, String>>>,
        command: Command,
        output: String,
        times: usize,
    ) -> MockExecute {
        self.get_output_for(envs, command, false, 0, output, times)
    }

    /// run command and return [anyhow::Result<Output>]
    pub fn get_output_for(
        mut self,
        envs: Option<Arc<BTreeMap<String, String>>>,
        command: Command,
        should_fail_to_execute: bool,
        exit_code: i32,
        output: String,
        times: usize,
    ) -> MockExecute {
        self.executor
            .expect_get_output()
            .with(eq(envs), eq(command))
            .returning(move |_, _| {
                if should_fail_to_execute {
                    bail!("failed to execute command");
                } else if exit_code != 0 {
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
            })
            .times(times);
        self.executor
    }
}
