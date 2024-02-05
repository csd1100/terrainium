use std::{fs::File, path::PathBuf, process::Command};

use anyhow::{Ok, Result};
use clap::Parser;
use terrainium::{
    handlers::helpers::get_process_log_file_path,
    types::executor::{Executable, ExecutorArgs, Status},
};

fn create_log_files(
    session_id: &String,
    process_uuid: &String,
) -> Result<(File, File, PathBuf, PathBuf)> {
    let out_file = format!("std_out-{}.log", process_uuid);
    let err_file = format!("std_err-{}.log", process_uuid);
    let out_file = &get_process_log_file_path(session_id, out_file)?;
    let err_file = &get_process_log_file_path(session_id, err_file)?;

    std::fs::write(&out_file, "")?;
    std::fs::write(&err_file, "")?;

    let out = File::options()
        .append(true)
        .create_new(true)
        .open(&out_file)?;
    let err = File::options()
        .append(true)
        .create_new(true)
        .open(&err_file)?;

    return Ok((out, err, out_file.to_path_buf(), err_file.to_path_buf()));
}

fn main() -> Result<()> {
    let cli = ExecutorArgs::parse();
    let string_arg = cli.exec;
    let command: Executable = serde_json::from_str(&string_arg)?;
    let (out, err, outfile_path, errfile_path) = create_log_files(&cli.id, &command.uuid)?;

    let mut cmd = Command::new(&command.exe);
    cmd.envs(std::env::vars()).stdout(out).stderr(err);

    if let Some(args) = command.args.clone() {
        cmd.args(&args);
    }

    let child = cmd.spawn().expect(&format!(
        "command {:?}, to start with args {:?}",
        &command.exe, &command.args
    ));

    let status_file = format!("status{}.json", command.uuid);
    let status_file = get_process_log_file_path(&cli.id, status_file)?;

    let child_stat = serde_json::to_string(&Status {
        uuid: command.uuid,
        pid: child.id(),
        cmd: command.exe,
        args: command.args,
        stdout_file: outfile_path,
        stderr_file: errfile_path,
        ec: None,
    })?;

    std::fs::write(&status_file, child_stat)?;

    let output = child.wait_with_output()?;

    // rewrite json with exit code status
    let stats = std::fs::read_to_string(&status_file)?;
    let mut stats: Status = serde_json::from_str(&stats)?;
    stats.ec = Some(output.status.to_string());
    let stats = serde_json::to_string(&stats)?;
    std::fs::write(&status_file, &stats)?;

    return Ok(());
}
