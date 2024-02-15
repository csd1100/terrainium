use std::{collections::HashMap, fs::File};

use anyhow::{anyhow, Result};

use crate::{
    handlers::{
        constants::{TERRAINIUM_EXECUTOR, TERRAINIUM_SESSION_ID},
        helpers::get_process_log_file_path,
    },
    types::{
        commands::{Command, Commands},
        executor::Executable,
    },
    shell::execute::Execute,
};

fn start_process_with_session_id(
    session_id: String,
    command: Command,
    envs: Option<HashMap<String, String>>,
) -> Result<()> {
    let exec_arg_json: Executable = command.into();
    let exec_arg = serde_json::to_string(&exec_arg_json)?;
    let mut command = TERRAINIUM_EXECUTOR;

    let dev = std::env::var("DEV");

    if dev.is_ok() && dev.unwrap() == *"true" {
        command = "./target/debug/terrainium_executor";
    }

    let args = vec!["--id", &session_id, "--exec", &exec_arg];

    let spawn_out_logs =
        get_process_log_file_path(&session_id, format!("spawn-out-{}.log", exec_arg_json.uuid))?;
    let spawn_err_logs =
        get_process_log_file_path(&session_id, format!("spawn-err-{}.log", exec_arg_json.uuid))?;

    let spawn_out = File::options()
        .append(true)
        .create_new(true)
        .open(spawn_out_logs)?;
    let spawn_err = File::options()
        .append(true)
        .create_new(true)
        .open(spawn_err_logs)?;

    Execute::spawn_and_get_child(command, args, envs, Some(spawn_out), Some(spawn_err))?;

    Ok(())
}

fn iterate_over_commands_and_spawn(
    session_id: &String,
    background: Vec<Command>,
    envs: HashMap<String, String>,
) -> Result<()> {
    let errors: Result<Vec<_>> = background
        .into_iter()
        .map(|command| {
            start_process_with_session_id(
                session_id.to_string(),
                command,
                Some(envs.clone()),
            )
        })
        .collect();

    if let Some(e) = errors.err() {
        Err(e)
    } else {
        Ok(())
    }
}

pub fn start_background_processes(
    commands: Option<Commands>,
    envs: &HashMap<String, String>,
) -> Result<()> {
    if let Some(commands) = commands {
        if let Some(background) = commands.background {
            if let Some(session_id) = envs.get(TERRAINIUM_SESSION_ID) {
                iterate_over_commands_and_spawn(session_id, background, envs.clone())?;
            } else if let Ok(session_id) = std::env::var(TERRAINIUM_SESSION_ID) {
                iterate_over_commands_and_spawn(&session_id, background, envs.clone())?;
            } else {
                return Err(anyhow!(
                    "Unable to get terrainium session id to start background processes"
                ));
            }
        }
    }
    Ok(())
}
