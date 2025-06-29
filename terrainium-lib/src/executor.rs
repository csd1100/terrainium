use std::collections::BTreeMap;
use std::process::Output;
use std::sync::Arc;

use anyhow::{Context, Result};

use crate::command::Command;

/// executes [Command]s
pub struct Executor;

pub trait Execute {
    fn get_output(
        &self,
        envs: Option<Arc<BTreeMap<String, String>>>,
        command: Command,
    ) -> Result<Output>;
}

impl Execute for Executor {
    /// get [Output] for [Command] executed with
    /// provided `envs`
    fn get_output(
        &self,
        envs: Option<Arc<BTreeMap<String, String>>>,
        command: Command,
    ) -> Result<Output> {
        let mut command: std::process::Command = command.into();
        if let Some(envs) = envs {
            command.envs(envs.as_ref());
        }
        command.output().context("failed to get output")
    }
}
