use crate::client::types::client::Client;
use crate::client::types::context::Context;
use anyhow::Result;

pub async fn handle(_context: Context, _client: Client) -> Result<()> {
    Ok(())
}
