use std::{
    collections::HashMap,
    process::{Child, Command, Output},
};

use anyhow::{Context, Ok, Result};

pub fn spawn_and_wait(
    command: &str,
    args: Vec<&str>,
    envs: Option<HashMap<String, String>>,
) -> Result<()> {
    if let Some(envs) = envs {
        let mut child_process = Command::new(command)
            .args(args.clone())
            .envs(&envs)
            .spawn()
            .context(format!(
                "Unable to execute command: {} with args: {:?} and env vars: {:?}",
                command, args, envs
            ))?;

        child_process.wait()?;
    } else {
        let mut child_process = Command::new(&command).args(&args).spawn().context(format!(
            "Unable to execute command: {} with args: {:?}",
            command, args
        ))?;

        child_process.wait()?;
    }

    return Ok(());
}

pub fn spawn_and_get_child(
    command: &str,
    args: Vec<&str>,
    envs: Option<HashMap<String, String>>,
) -> Result<Child> {
    if let Some(envs) = envs {
        let child_process = Command::new(command)
            .args(args.clone())
            .envs(&envs)
            .spawn()
            .context(format!(
                "Unable to execute command: {} with args: {:?} and env vars: {:?}",
                command, args, envs
            ))?;

        return Ok(child_process);
    } else {
        let child_process = Command::new(&command).args(&args).spawn().context(format!(
            "Unable to execute command: {} with args: {:?}",
            command, args
        ))?;

        return Ok(child_process);
    }
}

pub fn run_and_get_output(
    command: &str,
    args: Vec<&str>,
    envs: Option<HashMap<String, String>>,
) -> Result<Output> {
    let output;
    if let Some(envs) = envs {
        output = Command::new(command)
            .args(args.clone())
            .envs(&envs)
            .output()
            .context(format!(
                "Unable to execute command: {} with args: {:?} and env vars: {:?}",
                command, args, envs
            ))?;
    } else {
        output = Command::new(&command)
            .args(&args)
            .output()
            .context(format!(
                "Unable to execute command: {} with args: {:?}",
                command, args
            ))?;
    }

    return Ok(output);
}
