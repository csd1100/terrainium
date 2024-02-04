use std::{env::Vars, process::Command};

use anyhow::{Context, Ok, Result};

pub fn spawn_and_wait(command: String, args: Vec<String>, envs: Vars) -> Result<()> {
    let mut child_process = Command::new(&command)
        .args(&args)
        .envs(envs)
        .spawn()
        .context(format!(
            "Unable to execute command: {} with args: {:?}",
            command, args
        ))?;

    child_process.wait()?;

    return Ok(());
}
