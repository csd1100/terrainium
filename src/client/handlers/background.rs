use crate::client::args::{option_string_from, BiomeArg};
use crate::client::types::commands::Commands;
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use crate::common::constants::CONSTRUCTORS;
use crate::common::types::pb;
use crate::common::types::pb::{Error, ExecuteRequest, ExecuteResponse};
use crate::common::types::socket::Socket;
use crate::common::utils::timestamp;
use anyhow::{anyhow, Context as AnyhowContext, Result};
use prost_types::Any;
use std::collections::BTreeMap;
use std::fs::read_to_string;

fn operation_from_string(op: &str) -> pb::Operation {
    if op == CONSTRUCTORS {
        pb::Operation::Constructors
    } else {
        pb::Operation::Destructors
    }
}

pub fn execute_request(
    context: &mut Context,
    operation: &str,
    terrain: &Terrain,
    biome_name: String,
    envs: BTreeMap<String, String>,
    get_commands: fn(&Terrain, &Option<String>) -> Result<Commands>,
    is_activate: bool,
) -> Result<ExecuteRequest> {
    let commands = get_commands(terrain, &Some(biome_name.clone()))
        .context(format!("failed to merge {}", operation))?;

    let commands: Vec<pb::Command> = commands
        .background()
        .iter()
        .map(|command| {
            let mut command: pb::Command = command.clone().into();
            command.envs = envs.clone();
            command
        })
        .collect();

    Ok(ExecuteRequest {
        terrain_name: context.name(),
        biome_name,
        toml_path: context
            .toml_path()
            .expect("to be present")
            .display()
            .to_string(),
        is_activate,
        timestamp: timestamp(),
        operation: i32::from(operation_from_string(operation)),
        commands,
    })
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

    let selected_biome = option_string_from(&biome_arg);

    let mut envs = terrain
        .merged_envs(&selected_biome)
        .context("failed to merge envs")?;
    let mut final_envs = context.terrainium_envs().clone();
    final_envs.append(&mut envs);

    let (selected_biome, _) = terrain.select_biome(&selected_biome)?;
    let request = execute_request(
        context,
        operation,
        &terrain,
        selected_biome,
        final_envs,
        get_commands,
        false,
    )
    .context("failed to convert commands to execute request")?;

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
