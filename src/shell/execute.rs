use std::{
    collections::HashMap, fs::File, process::{Child, Command, Output}
};

use anyhow::{Context, Ok, Result};

pub fn spawn_and_wait(
    exe: &str,
    args: Vec<&str>,
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
    return Ok(());
}

pub fn spawn_and_get_child(
    exe: &str,
    args: Vec<&str>,
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
    return Ok(child_process);
}

pub fn run_and_get_output(
    exe: &str,
    args: Vec<&str>,
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
    return Ok(output);
}
