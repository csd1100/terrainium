use std::collections::HashMap;

use anyhow::Result;

use crate::types::commands::Commands;

pub fn start_background_processes(
    commands: Option<Commands>,
    _envs: Option<HashMap<String, String>>,
) -> Result<()> {
    if let Some(commands) = commands {
        if let Some(_background) = commands.background {}
    }
    return Ok(());
}
