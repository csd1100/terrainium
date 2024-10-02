use crate::client::args::{option_string_from, BiomeArg};
use crate::client::types::commands::Commands;
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use crate::common::constants::CONSTRUCTORS;
use crate::common::types::pb;
use crate::common::types::pb::{Error, ExecuteRequest, ExecuteResponse};
use crate::common::types::socket::Socket;
use anyhow::{anyhow, Context as AnyhowContext, Result};
use prost_types::Any;
use std::fs::read_to_string;

fn operation_from_string(op: &str) -> pb::Operation {
    if op == CONSTRUCTORS {
        pb::Operation::Constructors
    } else {
        pb::Operation::Destructors
    }
}

pub async fn handle(
    context: &mut Context,
    operation: &str,
    get_commands: fn(&Terrain, &Option<String>) -> Result<Commands>,
    biome_arg: Option<BiomeArg>,
) -> Result<()> {
    let terrain = Terrain::from_toml(
        read_to_string(context.toml_path()?).context("failed to read terrain.toml")?,
    )
    .expect("terrain to be parsed from toml");

    let commands = get_commands(&terrain, &option_string_from(&biome_arg))
        .context(format!("failed to merge {}", operation))?;

    let mut envs = terrain
        .merged_envs(&option_string_from(&biome_arg))
        .context("failed to merge envs")?;

    let mut final_envs = context.terrainium_envs().clone();
    final_envs.append(&mut envs);

    let commands: Vec<pb::Command> = commands
        .background()
        .iter()
        .map(|command| {
            let mut command: pb::Command = command.clone().into();
            command.envs = final_envs.clone();
            command
        })
        .collect();

    let request = ExecuteRequest {
        terrain_name: context.name(),
        operation: i32::from(operation_from_string(operation)),
        commands,
    };

    let client = context.socket();

    client
        .write_and_stop(Any::from_msg(&request).unwrap())
        .await?;

    let response: Any = client.read().await?;
    let execute_response: Result<ExecuteResponse> =
        Any::to_msg(&response).context("failed to convert to execute response from Any");

    if execute_response.is_ok() {
        println!("Success");
    } else {
        let error: Error = Any::to_msg(&response).context("failed to convert to error from Any")?;
        return Err(anyhow!(
            "error response from daemon {}",
            error.error_message
        ));
    }
    Ok(())
}
