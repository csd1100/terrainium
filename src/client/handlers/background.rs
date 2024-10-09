use crate::client::args::{option_string_from, BiomeArg};
#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use crate::common::constants::{
    CONSTRUCTORS, TERRAINIUMD_SOCKET, TERRAIN_SELECTED_BIOME, TERRAIN_SESSION_ID,
};
use crate::common::types::pb;
use crate::common::types::pb::{Error, ExecuteRequest, ExecuteResponse};
use crate::common::types::socket::Socket;
use crate::common::utils::timestamp;
use anyhow::{anyhow, Context as AnyhowContext, Result};
use prost_types::Any;
use std::collections::BTreeMap;
use std::fs::read_to_string;
use std::path::PathBuf;

fn operation_from_string(op: &str) -> pb::Operation {
    if op == CONSTRUCTORS {
        pb::Operation::Constructors
    } else {
        pb::Operation::Destructors
    }
}

pub async fn handle(
    context: &Context,
    operation: &str,
    biome_arg: Option<BiomeArg>,
    activate_envs: Option<BTreeMap<String, String>>,
    client: Option<Client>,
) -> Result<()> {
    let terrain = Terrain::from_toml(
        read_to_string(context.toml_path()?).context("failed to read terrain.toml")?,
    )
    .expect("terrain to be parsed from toml");

    let selected_biome = option_string_from(&biome_arg);

    let commands = if operation == CONSTRUCTORS {
        terrain
            .merged_constructors(&selected_biome)
            .context(format!("failed to merge {}", operation))?
    } else {
        terrain
            .merged_destructors(&selected_biome)
            .context(format!("failed to merge {}", operation))?
    };

    if commands.background().is_empty() {
        return Ok(());
    }

    let mut client = if let Some(client) = client {
        client
    } else {
        Client::new(PathBuf::from(TERRAINIUMD_SOCKET)).await?
    };

    let (biome_name, _) = terrain.select_biome(&selected_biome)?;

    let mut envs = terrain
        .merged_envs(&selected_biome)
        .context("failed to merge envs")?;
    envs.append(&mut context.terrainium_envs().clone());
    envs.insert(TERRAIN_SELECTED_BIOME.to_string(), biome_name.clone());

    if let Some(zsh_envs) = &activate_envs {
        envs.append(&mut zsh_envs.clone());
    } else {
        envs.remove(TERRAIN_SESSION_ID);
    }

    let commands: Vec<pb::Command> = commands
        .background()
        .iter()
        .map(|command| {
            let mut command: pb::Command = command.clone().into();
            command.envs = envs.clone();
            command
        })
        .collect();

    let session_id = if activate_envs.is_some() {
        context.session_id().to_string()
    } else {
        "".to_string()
    };

    let request = ExecuteRequest {
        session_id,
        terrain_name: context.name(),
        biome_name,
        toml_path: context
            .toml_path()
            .expect("to be present")
            .display()
            .to_string(),
        is_activate: activate_envs.is_some(),
        timestamp: timestamp(),
        operation: i32::from(operation_from_string(operation)),
        commands,
    };

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
