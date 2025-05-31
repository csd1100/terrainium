#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::environment::Environment;
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
    environment: Environment,
    activate_envs: Option<BTreeMap<String, String>>,
    client: Option<Client>,
) -> Result<()> {
    let commands = if operation == CONSTRUCTORS {
        environment.constructors()
    } else {
        environment.destructors()
    };

    if commands.background().is_empty() {
        return Ok(());
    }

    let mut client = if let Some(client) = client {
        client
    } else {
        Client::new(PathBuf::from(TERRAINIUMD_SOCKET)).await?
    };

    let mut envs = environment.envs();
    envs.append(&mut context.terrainium_envs().clone());
    envs.insert(
        TERRAIN_SELECTED_BIOME.to_string(),
        environment.selected_biome().to_string(),
    );

    if let Some(zsh_envs) = &activate_envs {
        envs.append(&mut zsh_envs.clone());
    } else {
        envs.remove(TERRAIN_SESSION_ID);
    }

    let commands: Result<Vec<pb::Command>> = commands
        .background()
        .iter()
        .map(|command| {
            let mut command: pb::Command = command.clone().try_into()?;
            command.envs = envs.clone();
            Ok(command)
        })
        .collect();

    let commands = commands.context("failed to convert background commands to protobuf message")?;

    let session_id = if activate_envs.is_some() {
        context.session_id().to_string()
    } else {
        "".to_string()
    };

    let request = ExecuteRequest {
        session_id,
        terrain_name: environment.name().to_string(),
        biome_name: environment.selected_biome().to_string(),
        toml_path: context.toml_path().display().to_string(),
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
