use crate::common::types::command::Command;
use anyhow::{Context, Result};
#[cfg(test)]
use mockall::mock;
use std::collections::BTreeMap;
use std::process::{ExitStatus, Output};
use std::sync::Arc;
use tracing::{info, trace};

pub trait Execute {
    fn get_output(
        &self,
        envs: Option<Arc<BTreeMap<String, String>>>,
        command: Command,
    ) -> Result<Output>;
    fn wait(
        &self,
        envs: Option<Arc<BTreeMap<String, String>>>,
        command: Command,
    ) -> Result<ExitStatus>;
    fn async_get_output(
        &self,
        envs: Option<Arc<BTreeMap<String, String>>>,
        command: Command,
    ) -> impl std::future::Future<Output = Result<Output>> + Send;
    fn async_spawn_with_log(
        &self,
        log_path: &str,
        envs: Option<Arc<BTreeMap<String, String>>>,
        command: Command,
    ) -> impl std::future::Future<Output = Result<ExitStatus>> + Send;
    fn async_spawn(
        &self,
        envs: Option<Arc<BTreeMap<String, String>>>,
        command: Command,
    ) -> impl std::future::Future<Output = Result<ExitStatus>> + Send;
}

#[derive(Default, Debug, PartialEq)]
pub struct Executor;

impl Execute for Executor {
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

    fn wait(
        &self,
        envs: Option<Arc<BTreeMap<String, String>>>,
        command: Command,
    ) -> Result<ExitStatus> {
        let mut command: std::process::Command = command.into();
        if let Some(envs) = envs {
            command.envs(envs.as_ref());
        }
        let mut child = command.spawn().context("failed to run command")?;
        child.wait().context("failed to wait for command")
    }

    async fn async_get_output(
        &self,
        envs: Option<Arc<BTreeMap<String, String>>>,
        command: Command,
    ) -> Result<Output> {
        info!("running async get_output for '{command}'");
        trace!("running async process {command:?} with envs {envs:?}");
        let mut command: tokio::process::Command = command.into();
        if let Some(envs) = envs {
            command.envs(envs.as_ref());
        }
        command.output().await.context("failed to get output")
    }

    async fn async_spawn_with_log(
        &self,
        log_path: &str,
        envs: Option<Arc<BTreeMap<String, String>>>,
        command: Command,
    ) -> Result<ExitStatus> {
        info!("running async process with wait for '{command}', with logs in file: {log_path}",);
        trace!("running async process with wait {command} and envs: {envs:?}");

        let log_file = tokio::fs::File::options()
            .create(true)
            .append(true)
            .open(log_path)
            .await
            .expect("failed to create / append to log file");

        let stdout: std::fs::File = log_file
            .try_clone()
            .await
            .expect("failed to clone file handle")
            .into_std()
            .await;

        let stderr: std::fs::File = log_file.into_std().await;

        let mut command: tokio::process::Command = command.into();
        if let Some(envs) = envs {
            command.envs(envs.as_ref());
        }
        command.stdout(stdout);
        command.stderr(stderr);
        let mut child = command.spawn().context("failed to run command")?;
        child.wait().await.context("failed to wait for command")
    }

    async fn async_spawn(
        &self,
        envs: Option<Arc<BTreeMap<String, String>>>,
        command: Command,
    ) -> Result<ExitStatus> {
        let mut command: tokio::process::Command = command.into();
        if let Some(envs) = envs {
            command.envs(envs.as_ref());
        }
        let mut child = command.spawn().context("failed to run command")?;
        child.wait().await.context("failed to wait for command")
    }
}

#[cfg(test)]
mock! {
    #[derive(Debug)]
    pub Executor {}

    impl Execute for Executor {
        fn get_output(&self, envs: Option<Arc<BTreeMap<String, String>>>, command: Command) -> Result<Output>;
        fn wait(&self, envs: Option<Arc<BTreeMap<String, String>>>, command: Command) -> Result<ExitStatus>;
        async fn async_get_output(
            &self,
            envs: Option<Arc<BTreeMap<String, String>>>,
            command: Command,
        ) -> Result<Output>;
        async fn async_spawn_with_log(
            &self,
            log_path: &str,
            envs: Option<Arc<BTreeMap<String, String>>>,
            command: Command,
        ) -> Result<ExitStatus>;
        async fn async_spawn(
            &self,
            envs: Option<Arc<BTreeMap<String, String>>>,
            command: Command,
        ) -> Result<ExitStatus>;
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::client::test_utils;
    use crate::common::execute::{Execute, Executor};
    use crate::common::types::command::Command;
    use anyhow::Result;
    use std::collections::BTreeMap;
    use std::env::VarError;
    use std::sync::Arc;

    #[test]
    fn test_spawn_and_get_output_without_envs() -> Result<()> {
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
    fn test_spawn_and_get_output_with_envs() -> Result<()> {
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
    fn test_run_set_args_and_envs() -> Result<()> {
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

    #[ignore]
    #[test]
    fn test_wait() -> Result<()> {
        let mut envs: BTreeMap<String, String> = BTreeMap::new();
        envs.insert(
            "TEST_SCRIPT".to_string(),
            "./tests/scripts/print_num_for_10_sec".to_string(),
        );

        let command = Command::new(
            "/bin/bash".to_string(),
            vec!["-c".to_string(), "$TEST_SCRIPT".to_string()],
            Some(std::env::current_dir()?),
        );

        let output = Executor
            .wait(Some(Arc::new(envs)), command)
            .expect("not to fail");

        assert_eq!(0, output.code().expect("to be present"));

        Ok(())
    }
}
