use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::common::constants::TERRAINIUM_ENABLED;
use crate::common::types::pb;
use crate::common::types::pb::Error;
use crate::common::types::socket::Socket;
use anyhow::{anyhow, Context as AnyhowContext, Result};
use prost_types::Any;
use std::env;

pub async fn handle(context: Context, mut client: Client) -> Result<()> {
    let is_terrain_enabled = env::var(TERRAINIUM_ENABLED).unwrap_or_else(|_| "false".to_string());

    if is_terrain_enabled != "true" {
        return Err(anyhow!(
            "no active terrain found, use `terrainium enter` command to activate a terrain."
        ));
    }

    let session_id = context.session_id();
    let terrain_name = context.name();

    let request = pb::StatusRequest {
        session_id,
        terrain_name,
    };

    client
        .write_and_stop(Any::from_msg(&request).unwrap())
        .await?;

    let response: Any = client.read().await?;

    let status_response: Result<pb::StatusResponse> = response
        .to_msg()
        .context("failed to convert status response from any");

    if let Ok(status) = status_response {
        println!("Success");
        println!("{:#?}", status);
    } else {
        let error: Error = response
            .to_msg()
            .context("failed to convert to error from Any")?;

        return Err(anyhow!(
            "error response from daemon {}",
            error.error_message
        ));
    }
    Ok(())
}
