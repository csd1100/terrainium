use std::{collections::HashMap, process::Command};

use anyhow::{Context, Ok, Result};

pub fn spawn_and_wait(
    command: String,
    args: Vec<String>,
    envs: Option<HashMap<String, String>>,
) -> Result<()> {
    if let Some(envs) = envs {
        let mut child_process = Command::new(&command)
            .args(&args)
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
