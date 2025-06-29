use std::collections::BTreeMap;
use std::fmt::Debug;
use std::process::Output;
use std::sync::Arc;

use anyhow::{Context, Result};
#[cfg(any(test, feature = "test-exports"))]
use mockall::automock;

use crate::command::Command;

/// executes [Command]s
#[derive(Debug)]
pub struct Executor;

#[cfg_attr(any(test, feature = "test-exports"), automock)]
pub trait Execute: Debug {
    fn get_output(
        &self,
        envs: Option<Arc<BTreeMap<String, String>>>,
        command: Command,
    ) -> Result<Output>;
}

impl Execute for Executor {
    /// get [Output] for [Command] executed with
    /// provided `envs`
    fn get_output(
        &self,
        envs: Option<Arc<BTreeMap<String, String>>>,
        command: Command,
    ) -> Result<Output> {
        let mut command: std::process::Command = command.into();
        if let Some(envs) = envs {
            command.envs(envs.as_ref());
        }
        command.output().context("failed to get output")
    }
}

#[cfg(test)]
mod tests {
    use std::env::VarError;

    use anyhow::Result;
    use serial_test::serial;

    use super::*;
    use crate::test_utils;

    #[test]
    fn test_get_output_without_envs() -> Result<()> {
        let test_var = "TEST_VAR";
        let orig_env: std::result::Result<String, VarError>;
        unsafe {
            orig_env = test_utils::set_env_var(test_var, Some("TEST_VALUE"));
        }

        let command = Command::new(
            "/bin/bash".to_string(),
            vec!["-c".to_string(), "echo $TEST_VAR".to_string()],
            Some(std::env::current_dir()?),
        );

        let output = Executor.get_output(None, command).expect("not to fail");

        assert_eq!(
            "TEST_VALUE\n",
            String::from_utf8(output.stdout).expect("convert to ascii")
        );

        unsafe {
            test_utils::restore_env_var(test_var, orig_env);
        }

        Ok(())
    }

    #[test]
    #[serial]
    fn test_get_output_with_envs() -> Result<()> {
        let test_var1 = "TEST_VAR1";
        let test_var2 = "TEST_VAR2";

        let orig_env1: std::result::Result<String, VarError>;
        let orig_env2: std::result::Result<String, VarError>;

        unsafe {
            orig_env1 = test_utils::set_env_var(test_var1, Some("OLD_VALUE1"));
            orig_env2 = test_utils::set_env_var(test_var2, Some("OLD_VALUE2"));
        }

        let mut envs: BTreeMap<String, String> = BTreeMap::new();
        envs.insert(test_var1.to_owned(), "NEW_VALUE1".to_string());

        let command = Command::new(
            "/bin/bash".to_string(),
            vec![
                "-c".to_string(),
                "echo \"$TEST_VAR1\n$TEST_VAR2\"".to_string(),
            ],
            Some(std::env::current_dir()?),
        );

        let output = Executor
            .get_output(Some(Arc::new(envs)), command)
            .expect("not to fail");

        assert_eq!(
            "NEW_VALUE1\nOLD_VALUE2\n",
            String::from_utf8(output.stdout).expect("convert to ascii")
        );

        unsafe {
            test_utils::restore_env_var(test_var1, orig_env1);
            test_utils::restore_env_var(test_var2, orig_env2);
        }
        Ok(())
    }

    #[test]
    #[serial]
    fn test_get_output_set_args_and_envs() -> Result<()> {
        let mut envs: BTreeMap<String, String> = BTreeMap::new();
        envs.insert("TEST_VAR".to_string(), "TEST_VALUE".to_string());

        let args: Vec<String> = vec!["-c".to_string(), "echo \"$TEST_VAR\"".to_string()];

        let mut command = Command::new(
            "/bin/bash".to_string(),
            vec![],
            Some(std::env::current_dir()?),
        );
        command.set_args(args);

        let output = Executor
            .get_output(Some(Arc::new(envs)), command)
            .expect("not to fail");

        assert_eq!(
            "TEST_VALUE\n",
            String::from_utf8(output.stdout).expect("convert to ascii")
        );

        Ok(())
    }
}
