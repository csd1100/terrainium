use anyhow::{Context, Result};
#[cfg(test)]
use mockall::mock;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::process::{Command, ExitStatus, Output};
use tokio::fs;
use tracing::{event, instrument, Level};

pub trait Execute {
    fn get_output(self) -> Result<Output>;
    fn wait(self) -> Result<ExitStatus>;
    fn async_get_output(self) -> impl std::future::Future<Output = Result<Output>> + Send;
    fn async_wait(
        self,
        log_path: &str,
    ) -> impl std::future::Future<Output = Result<ExitStatus>> + Send;
    fn async_spawn(self) -> impl std::future::Future<Output = Result<ExitStatus>> + Send;
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct CommandToRun {
    exe: String,
    args: Vec<String>,
    envs: Option<BTreeMap<String, String>>,
}

impl CommandToRun {
    pub fn new(exe: String, args: Vec<String>, envs: Option<BTreeMap<String, String>>) -> Self {
        CommandToRun { exe, args, envs }
    }

    pub fn set_args(&mut self, args: Vec<String>) {
        self.args = args;
    }

    pub fn set_envs(&mut self, envs: Option<BTreeMap<String, String>>) {
        self.envs = envs;
    }
}

impl From<CommandToRun> for Command {
    fn from(value: CommandToRun) -> Command {
        let mut vars: BTreeMap<String, String> = std::env::vars().collect();
        let envs = if let Some(mut envs) = value.envs {
            vars.append(&mut envs);
            vars
        } else {
            vars
        };
        let mut command = Command::new(value.exe);
        command.args(value.args).envs(envs);
        command
    }
}

impl From<CommandToRun> for tokio::process::Command {
    fn from(value: CommandToRun) -> tokio::process::Command {
        let mut vars: BTreeMap<String, String> = std::env::vars().collect();
        let envs = if let Some(mut envs) = value.envs {
            vars.append(&mut envs);
            vars
        } else {
            vars
        };
        let mut command = tokio::process::Command::new(value.exe);
        command.args(value.args).envs(envs);
        command
    }
}

impl Execute for CommandToRun {
    fn get_output(self) -> Result<Output> {
        let mut command: Command = self.into();
        command.output().context("failed to get output")
    }

    fn wait(self) -> Result<ExitStatus> {
        let mut command: Command = self.into();
        let mut child = command.spawn().context("failed to run command")?;
        child.wait().context("failed to wait for command")
    }

    #[instrument]
    async fn async_get_output(self) -> Result<Output> {
        event!(Level::INFO, "running async get_output for {:?}", self);
        let mut command: tokio::process::Command = self.into();
        command.output().await.context("failed to get output")
    }

    async fn async_wait(self, log_path: &str) -> Result<ExitStatus> {
        event!(
            Level::INFO,
            "running async process with wait for {:?}, with logs in file: {:?}",
            &self,
            log_path,
        );

        let log_file = fs::File::options()
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

        let mut command: tokio::process::Command = self.into();
        command.stdout(stdout);
        command.stderr(stderr);
        let mut child = command.spawn().context("failed to run command")?;
        child.wait().await.context("failed to wait for command")
    }

    async fn async_spawn(self) -> Result<ExitStatus> {
        let mut command: tokio::process::Command = self.into();
        let mut child = command.spawn().context("failed to run command")?;
        child.wait().await.context("failed to wait for command")
    }
}

#[cfg(test)]
mock! {
    #[derive(Debug)]
    pub CommandToRun {
        pub fn new(exe: String, args: Vec<String>, envs: Option<BTreeMap<String, String>>) -> Self;
        pub fn set_args(&mut self, args: Vec<String>);
        pub fn set_envs(&mut self, envs: Option<BTreeMap<String, String>>);
    }

    impl Execute for CommandToRun {
        fn get_output(self) -> Result<Output>;
        fn wait(self) -> Result<ExitStatus>;
        async fn async_get_output(self) -> Result<Output>;
        async fn async_wait(self, log_path: &str) -> Result<ExitStatus>;
        async fn async_spawn(self) -> Result<ExitStatus>;
    }

    impl Clone for CommandToRun {
        fn clone(&self) -> Self;
    }

    impl PartialEq for CommandToRun {
        fn eq(&self, other: &Self) -> bool;
    }
}

#[cfg(test)]
pub(crate) mod test {
    use crate::common::execute::{CommandToRun, Execute};
    use anyhow::Result;
    use std::collections::BTreeMap;
    use std::env::VarError;

    #[test]
    fn test_spawn_and_get_output_without_envs() -> Result<()> {
        let test_var = "TEST_VAR".to_string();
        let orig_env = set_env_var(test_var.clone(), Some("TEST_VALUE".to_string()));

        let run = CommandToRun::new(
            "/bin/bash".to_string(),
            vec!["-c".to_string(), "echo $TEST_VAR".to_string()],
            None,
        );

        let output = run.get_output().expect("not to fail");

        assert_eq!(
            "TEST_VALUE\n",
            String::from_utf8(output.stdout).expect("convert to ascii")
        );

        restore_env_var(test_var.clone(), orig_env);

        Ok(())
    }

    #[test]
    fn test_spawn_and_get_output_with_envs() -> Result<()> {
        let test_var1: String = "TEST_VAR1".to_string();
        let test_var2 = "TEST_VAR2".to_string();

        let orig_env1 = set_env_var(test_var1.clone(), Some("OLD_VALUE1".to_string()));
        let orig_env2 = set_env_var(test_var2.clone(), Some("OLD_VALUE2".to_string()));

        let mut envs: BTreeMap<String, String> = BTreeMap::new();
        envs.insert(test_var1.clone(), "NEW_VALUE1".to_string());

        let run = CommandToRun::new(
            "/bin/bash".to_string(),
            vec![
                "-c".to_string(),
                "echo \"$TEST_VAR1\n$TEST_VAR2\"".to_string(),
            ],
            Some(envs),
        );

        let output = run.get_output().expect("not to fail");

        assert_eq!(
            "NEW_VALUE1\nOLD_VALUE2\n",
            String::from_utf8(output.stdout).expect("convert to ascii")
        );

        restore_env_var(test_var1, orig_env1);
        restore_env_var(test_var2, orig_env2);

        Ok(())
    }

    #[test]
    fn test_run_set_args_and_envs() -> Result<()> {
        let test_var = "TEST_VAR".to_string();

        let mut envs: BTreeMap<String, String> = BTreeMap::new();
        envs.insert(test_var.clone(), "TEST_VALUE".to_string());

        let args: Vec<String> = vec!["-c".to_string(), "echo \"$TEST_VAR\"".to_string()];

        let mut run = CommandToRun::new("/bin/bash".to_string(), vec![], None);
        run.set_envs(Some(envs));
        run.set_args(args);

        let output = run.get_output().expect("not to fail");

        assert_eq!(
            "TEST_VALUE\n",
            String::from_utf8(output.stdout).expect("convert to ascii")
        );

        Ok(())
    }

    #[ignore]
    #[test]
    fn test_wait() -> Result<()> {
        let script = "TEST_SCRIPT".to_string();

        let mut envs: BTreeMap<String, String> = BTreeMap::new();
        envs.insert(
            script.clone(),
            "./tests/scripts/print_num_for_10_sec".to_string(),
        );

        let run = CommandToRun::new(
            "/bin/bash".to_string(),
            vec!["-c".to_string(), "$TEST_SCRIPT".to_string()],
            Some(envs),
        );

        let output = run.wait().expect("not to fail");

        assert_eq!(0, output.code().expect("to be present"));

        Ok(())
    }

    pub fn set_env_var(
        key: String,
        value: Option<String>,
    ) -> std::result::Result<String, VarError> {
        // FIX: the tests run in parallel so setting same env var will cause tests to fail
        // as env var is not reset yet
        let orig_env = std::env::var(&key);
        if let Some(val) = value {
            std::env::set_var(&key, val);
        } else {
            std::env::remove_var(&key);
        }

        orig_env
    }

    pub fn restore_env_var(key: String, orig_env: Result<String, VarError>) {
        // FIX: the tests run in parallel so restoring env vars won't help if vars have same key
        if let Ok(orig_var) = orig_env {
            std::env::set_var(&key, &orig_var);
            assert!(std::env::var(&key).is_ok());
            assert_eq!(orig_var, std::env::var(&key).expect("var to be present"));
        } else {
            std::env::remove_var(&key);
            assert!(std::env::var(&key).is_err());
        }
    }
}
