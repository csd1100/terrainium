use clap::Parser;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;
use std::path::PathBuf;

use super::commands::Command;

#[derive(Serialize, Deserialize)]
pub struct Status {
    pub uuid: String,
    pub pid: u32,
    pub cmd: String,
    pub args: Option<Vec<String>>,
    pub stdout_file: PathBuf,
    pub stderr_file: PathBuf,
    pub ec: Option<String>,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct ExecutorArgs {
    #[arg(long)]
    pub id: String,

    #[arg(long)]
    pub exec: String,
}

#[derive(Serialize, Deserialize)]
pub struct Executable {
    pub uuid: String,
    pub exe: String,
    pub args: Option<Vec<String>>,
}

impl From<Command> for Executable {
    fn from(value: Command) -> Self {
        return Executable {
            uuid: Uuid::new_v4().to_string(),
            exe: value.exe,
            args: value.args,
        };
    }
}
