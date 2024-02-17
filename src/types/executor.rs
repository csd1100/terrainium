use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

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
        Executable {
            uuid: Uuid::new_v4().to_string(),
            exe: value.exe,
            args: value.args,
        }
    }
}

impl Executable {
    pub fn get_uuid(&self) -> String {
        self.uuid.to_string()
    }
}

#[cfg(test)]
use mockall::mock;

#[cfg(test)]
use serde::Deserializer;

#[cfg(test)]
mock! {
    pub Executable {
        pub fn get_uuid(&self) -> String;
        pub fn private_deserialize(deserializable: Result<Executable, ()>) -> Self;
        pub fn private_serialize(&self) -> Executable;
    }

    impl From<Command> for Executable {
        fn from(value: Command) -> Self;
    }
}

#[cfg(test)]
// Manually implement Serialize for MockExecutable
impl serde::Serialize for MockExecutable {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.private_serialize().serialize(s)
    }
}

#[cfg(test)]
// Manually implement Deserialize for MockExecutable
impl<'de> Deserialize<'de> for MockExecutable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let serializable = Executable::deserialize(deserializer).map_err(|_| ());
        Ok(MockExecutable::private_deserialize(serializable))
    }
}
