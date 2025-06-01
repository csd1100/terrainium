#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::environment::Environment;
use anyhow::Result;
use std::collections::BTreeMap;

pub async fn handle(
    _context: &Context,
    _operation: &str,
    _environment: Environment,
    _activate_envs: Option<BTreeMap<String, String>>,
    _client: Option<Client>,
) -> Result<()> {
    Ok(())
}
