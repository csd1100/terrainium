#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub mod spawn {
    use std::{
        collections::HashMap,
        fs::File,
        process::{Child, Command, Output},
    };

    use anyhow::{Context, Ok, Result};

    #[allow(clippy::needless_lifetimes)]
    pub fn and_wait<'a>(
        exe: &str,
        args: Vec<&'a str>,
        envs: Option<HashMap<String, String>>,
    ) -> Result<()> {
        let mut command = Command::new(exe);
        command.args(args.clone());
        if let Some(envs) = &envs {
            command.envs(envs);
        }
        let mut child_process = command.spawn().context(format!(
            "Unable to execute command: {} with args: {:?} and env vars: {:?}",
            exe, args, envs
        ))?;
        child_process.wait()?;
        Ok(())
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn and_get_child<'a>(
        exe: &str,
        args: Vec<&'a str>,
        envs: Option<HashMap<String, String>>,
        stdout: Option<File>,
        stderr: Option<File>,
    ) -> Result<Child> {
        let mut command = Command::new(exe);
        command.args(args.clone());
        if let Some(envs) = &envs {
            command.envs(envs);
        }
        if let Some(stdout) = stdout {
            command.stdout(stdout);
        }
        if let Some(stderr) = stderr {
            command.stderr(stderr);
        }
        let child_process = command.spawn().context(format!(
            "Unable to execute command: {} with args: {:?} and env vars: {:?}",
            exe, args, envs
        ))?;
        Ok(child_process)
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn and_get_output<'a>(
        exe: &str,
        args: Vec<&'a str>,
        envs: Option<HashMap<String, String>>,
    ) -> Result<Output> {
        let mut command = Command::new(exe);
        command.args(args.clone());
        if let Some(envs) = &envs {
            command.envs(envs);
        }
        let output = command.output().context(format!(
            "Unable to execute command: {} with args: {:?} and env vars: {:?}",
            exe, args, envs
        ))?;
        Ok(output)
    }
}
