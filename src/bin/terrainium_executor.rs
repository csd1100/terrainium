use std::{fs::File, path::PathBuf, process::Command};

use anyhow::{Context, Ok, Result};
use clap::Parser;
use terrainium::{
    helpers::helpers::get_process_log_file_path,
    types::executor::{Executable, ExecutorArgs, Status},
};

fn create_log_file(session_id: &String, filename: String) -> Result<(File, PathBuf)> {
    let out_file = &get_process_log_file_path(session_id, filename.clone())
        .context(format!("Unable to get log file path: {}", filename))?;
    let out = File::options()
        .append(true)
        .create_new(true)
        .open(out_file)
        .context(format!("Unable to create and open file:{:?}", &out_file))?;
    Ok((out, out_file.to_path_buf()))
}

fn create_log_files(
    session_id: &String,
    process_uuid: &String,
) -> Result<((File, PathBuf), (File, PathBuf))> {
    let out_file = format!("std_out-{}.log", process_uuid);
    let err_file = format!("std_err-{}.log", process_uuid);
    Ok((
        create_log_file(session_id, out_file)?,
        create_log_file(session_id, err_file)?,
    ))
}

fn main() -> Result<()> {
    let cli = ExecutorArgs::parse();
    let string_arg = cli.exec;
    let command: Executable = serde_json::from_str(&string_arg)?;
    let ((out, outfile_path), (err, errfile_path)) = create_log_files(&cli.id, &command.uuid)
        .context("Unable to get stdout and stderr files")?;

    let mut cmd = Command::new(&command.exe);
    cmd.envs(std::env::vars()).stdout(out).stderr(err);

    if let Some(args) = command.args.clone() {
        cmd.args(&args);
    }

    let child = cmd.spawn().unwrap_or_else(|_| {
        panic!(
            "command {:?}, to start with args {:?}",
            &command.exe, &command.args
        )
    });

    let status_file_name = format!("status-{}.json", command.uuid);
    let status_file_path = get_process_log_file_path(&cli.id, status_file_name.clone())?;
    {
        let (status_file, _) = create_log_file(&cli.id, status_file_name)?;

        serde_json::to_writer_pretty(
            &status_file,
            &Status {
                uuid: command.uuid,
                pid: child.id(),
                cmd: command.exe,
                args: command.args,
                stdout_file: outfile_path,
                stderr_file: errfile_path,
                ec: None,
            },
        )?;
    }

    let output = child.wait_with_output()?;

    // rewrite json with exit code status
    let mut existing_status: Status;
    {
        let status_file = File::open(&status_file_path).context(format!(
            "Unable to open file:{:?}, for reading",
            &status_file_path
        ))?;
        existing_status = serde_json::from_reader(status_file)?;
    }

    existing_status.ec = Some(output.status.to_string());

    let status_file = File::options()
        .write(true)
        .open(&status_file_path)
        .context(format!(
            "Unable to open file:{:?} to update exit code",
            &status_file_path
        ))?;
    serde_json::to_writer_pretty(&status_file, &existing_status)?;

    Ok(())
}
