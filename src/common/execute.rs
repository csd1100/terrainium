use crate::common::types::pb;
use anyhow::{Context, Result};
#[cfg(test)]
use mockall::mock;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};
use tokio::fs;
use tracing::{info, instrument, trace};

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
    cwd: PathBuf,
}

impl Display for CommandToRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} in {}",
            self.exe,
            self.args.join(" "),
            self.cwd.display()
        )
    }
}

impl CommandToRun {
    pub fn new(
        exe: String,
        args: Vec<String>,
        envs: Option<BTreeMap<String, String>>,
        cwd: &Path,
    ) -> Self {
        let cwd = cwd.to_path_buf();
        CommandToRun {
            exe,
            args,
            envs,
            cwd,
        }
    }

    pub fn exe(&self) -> &str {
        &self.exe
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    pub fn set_args(&mut self, args: Vec<String>) {
        self.args = args;
    }

    pub fn set_envs(&mut self, envs: Option<BTreeMap<String, String>>) {
        self.envs = envs;
    }
}

impl From<CommandToRun> for std::process::Command {
    fn from(value: CommandToRun) -> std::process::Command {
        let mut vars: BTreeMap<String, String> = std::env::vars().collect();
        let envs = if let Some(mut envs) = value.envs {
            vars.append(&mut envs);
            vars
        } else {
            vars
        };
        let mut command = std::process::Command::new(value.exe);
        command.args(value.args).envs(envs).current_dir(value.cwd);
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
        command.args(value.args).envs(envs).current_dir(value.cwd);
        command
    }
}

impl From<CommandToRun> for pb::Command {
    fn from(value: CommandToRun) -> Self {
        let CommandToRun {
            exe,
            args,
            envs,
            cwd,
        } = value;
        Self {
            exe,
            args,
            envs: envs.unwrap_or_default(),
            cwd: cwd.to_string_lossy().to_string(),
        }
    }
}

impl Execute for CommandToRun {
    fn get_output(self) -> Result<Output> {
        let mut command: std::process::Command = self.into();
        command.output().context("failed to get output")
    }

    fn wait(self) -> Result<ExitStatus> {
        let mut command: std::process::Command = self.into();
        let mut child = command.spawn().context("failed to run command")?;
        child.wait().context("failed to wait for command")
    }

    #[instrument]
    async fn async_get_output(self) -> Result<Output> {
        info!("running async get_output for '{self}'");
        trace!("running async process {self:?}");
        let mut command: tokio::process::Command = self.into();
        command.output().await.context("failed to get output")
    }

    async fn async_wait(self, log_path: &str) -> Result<ExitStatus> {
        info!("running async process with wait for '{self}', with logs in file: {log_path}",);
        trace!("running async process with wait {self:?}");

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

impl From<pb::Command> for CommandToRun {
    fn from(value: pb::Command) -> Self {
        Self {
            exe: value.exe,
            args: value.args,
            envs: Some(value.envs),
            cwd: PathBuf::from(value.cwd),
        }
    }
}

#[cfg(test)]
mock! {
    #[derive(Debug)]
    pub CommandToRun {
        pub fn new(exe: String, args: Vec<String>, envs: Option<BTreeMap<String, String>>, cwd: &Path) -> Self;
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
pub(crate) mod tests {
    use crate::client::test_utils;
    use crate::common::execute::{CommandToRun, Execute};
    use anyhow::Result;
    use std::collections::BTreeMap;

    #[test]
    fn test_spawn_and_get_output_without_envs() -> Result<()> {
        let test_var = "TEST_VAR".to_string();
        let orig_env = test_utils::set_env_var(test_var.clone(), Some("TEST_VALUE".to_string()));

        let run = CommandToRun::new(
            "/bin/bash".to_string(),
            vec!["-c".to_string(), "echo $TEST_VAR".to_string()],
            None,
            &std::env::current_dir()?,
        );

        let output = run.get_output().expect("not to fail");

        assert_eq!(
            "TEST_VALUE\n",
            String::from_utf8(output.stdout).expect("convert to ascii")
        );

        test_utils::restore_env_var(test_var.clone(), orig_env);

        Ok(())
    }

    #[test]
    fn test_spawn_and_get_output_with_envs() -> Result<()> {
        let test_var1: String = "TEST_VAR1".to_string();
        let test_var2 = "TEST_VAR2".to_string();

        let orig_env1 = test_utils::set_env_var(test_var1.clone(), Some("OLD_VALUE1".to_string()));
        let orig_env2 = test_utils::set_env_var(test_var2.clone(), Some("OLD_VALUE2".to_string()));

        let mut envs: BTreeMap<String, String> = BTreeMap::new();
        envs.insert(test_var1.clone(), "NEW_VALUE1".to_string());

        let run = CommandToRun::new(
            "/bin/bash".to_string(),
            vec![
                "-c".to_string(),
                "echo \"$TEST_VAR1\n$TEST_VAR2\"".to_string(),
            ],
            Some(envs),
            &std::env::current_dir()?,
        );

        let output = run.get_output().expect("not to fail");

        assert_eq!(
            "NEW_VALUE1\nOLD_VALUE2\n",
            String::from_utf8(output.stdout).expect("convert to ascii")
        );

        test_utils::restore_env_var(test_var1, orig_env1);
        test_utils::restore_env_var(test_var2, orig_env2);

        Ok(())
    }

    #[test]
    fn test_run_set_args_and_envs() -> Result<()> {
        let test_var = "TEST_VAR".to_string();

        let mut envs: BTreeMap<String, String> = BTreeMap::new();
        envs.insert(test_var.clone(), "TEST_VALUE".to_string());

        let args: Vec<String> = vec!["-c".to_string(), "echo \"$TEST_VAR\"".to_string()];

        let mut run = CommandToRun::new(
            "/bin/bash".to_string(),
            vec![],
            None,
            &std::env::current_dir()?,
        );
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
            &std::env::current_dir()?,
        );

        let output = run.wait().expect("not to fail");

        assert_eq!(0, output.code().expect("to be present"));

        Ok(())
    }
}
