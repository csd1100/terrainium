use crate::common::execute::{CommandToRun, Execute};
use crate::common::types::pb;
use crate::common::types::pb::response::Payload::{Body, Error};
use crate::common::types::pb::{Construct, Response};
use crate::common::utils::remove_non_numeric;
use crate::daemon::handlers::RequestHandler;
use crate::daemon::types::context::DaemonContext;
use crate::daemon::types::state_manager::StateManager;
use crate::daemon::types::terrain_state::{CommandState, CommandStatus, TerrainState};
use anyhow::{bail, Context, Result};
use prost_types::Any;
use std::sync::Arc;
use tracing::{debug, error, trace};

pub(crate) struct ConstructHandler;

impl RequestHandler for ConstructHandler {
    async fn handle(request: Any, context: DaemonContext) -> Any {
        trace!("handling Construct request");
        let activate: Result<Construct> = request
            .to_msg()
            .context("failed to convert request to Activate");

        trace!("result of attempting to parse request: {:#?}", activate);

        let response = match activate {
            Ok(construct) => {
                let result = spawn_constructors(construct, context).await;
                if let Err(err) = result {
                    Response {
                        payload: Some(Error(err.to_string())),
                    }
                } else {
                    Response {
                        payload: Some(Body(pb::Body { message: None })),
                    }
                }
            }
            Err(err) => Response {
                payload: Some(Error(err.to_string())),
            },
        };
        Any::from_msg(&response).unwrap()
    }
}

pub(crate) async fn spawn_constructors(
    constructors: Construct,
    context: DaemonContext,
) -> Result<()> {
    let Construct {
        session_id,
        terrain_name,
        biome_name,
        toml_path,
        timestamp,
        commands,
    } = constructors;

    let current_timestamp = timestamp.clone();

    let state = if session_id.is_none() {
        // session_id is not provided that means running constructors outside
        // terrainium shell so create a new state
        let state: TerrainState = TerrainState::from_construct_destruct(
            session_id,
            terrain_name,
            biome_name,
            toml_path,
            timestamp,
            true,
            commands,
        );
        context.state_manager().create_state(&state).await?;

        state
    } else {
        // if session_id is present check if CommandStatus is present for current
        // timestamp else add new entry
        let session_id = session_id.unwrap();
        let numeric_timestamp = remove_non_numeric(&timestamp);

        let commands = commands
            .clone()
            .into_iter()
            .enumerate()
            .map(|(idx, cmd)| {
                CommandState::from(&terrain_name, &session_id, idx, &numeric_timestamp, cmd)
            })
            .collect();

        context
            .state_manager()
            .add_commands_if_necessary(&terrain_name, &session_id, &timestamp, true, commands)
            .await?;

        context
            .state_manager()
            .fetch_state(&terrain_name, &session_id)
            .await?
    };

    let TerrainState {
        session_id,
        terrain_name,
        mut constructors,
        ..
    } = state;

    let command_states = constructors.remove(&current_timestamp).unwrap();

    command_states
        .into_iter()
        .enumerate()
        .for_each(|(index, cmd_state)| {
            let CommandState {
                command, log_path, ..
            } = cmd_state;
            let terrain_name = terrain_name.clone();
            let session_id = session_id.clone();
            let timestamp = current_timestamp.clone();
            let state_manager = context.state_manager();
            let log_path = log_path.clone();
            tokio::spawn(async move {
                let res = spawn_command(
                    terrain_name,
                    session_id,
                    true,
                    timestamp,
                    index,
                    log_path,
                    state_manager,
                    command.clone(),
                )
                .await;

                if let Err(err) = res {
                    error!("failed to spawn command: {:?}", err);
                }
            });
        });

    Ok(())
}

async fn spawn_command(
    terrain_name: String,
    session_id: String,
    is_constructor: bool,
    timestamp: String,
    index: usize,
    log_path: String,
    state_manager: Arc<StateManager>,
    command: CommandToRun,
) -> Result<()> {
    let cmd_str = command.to_string();
    trace!(
        terrain_name = terrain_name,
        session_id = session_id,
        is_constructor = is_constructor,
        timestamp = timestamp,
        index = index,
        "running command {cmd_str}"
    );

    debug!(
        terrain_name = terrain_name,
        session_id = session_id,
        is_constructor = is_constructor,
        timestamp = timestamp,
        index = index,
        "setting command status to running..."
    );

    state_manager
        .update_command_status(
            &terrain_name,
            &session_id,
            &timestamp,
            index,
            is_constructor,
            CommandStatus::Running,
        )
        .await?;

    let res = command.async_wait(&log_path).await;

    match res {
        Ok(exit_status) => {
            if exit_status.success() {
                state_manager
                    .update_command_status(
                        &terrain_name,
                        &session_id,
                        &timestamp,
                        index,
                        is_constructor,
                        CommandStatus::Succeeded,
                    )
                    .await?;
                debug!(
                    terrain_name = terrain_name,
                    session_id = session_id,
                    is_constructor = is_constructor,
                    timestamp = timestamp,
                    index = index,
                    "command `{cmd_str}` completed successfully"
                );
            } else {
                state_manager
                    .update_command_status(
                        &terrain_name,
                        &session_id,
                        &timestamp,
                        index,
                        is_constructor,
                        CommandStatus::Failed(exit_status.code()),
                    )
                    .await?;

                let error = format!(
                    "command: `{cmd_str}` exited with code {:?}",
                    exit_status.code()
                );
                error!(
                    terrain_name = terrain_name,
                    session_id = session_id,
                    is_constructor = is_constructor,
                    timestamp = timestamp,
                    index = index,
                    "{error}"
                );
                bail!(error);
            }
        }
        Err(err) => {
            state_manager
                .update_command_status(
                    &terrain_name,
                    &session_id,
                    &timestamp,
                    index,
                    is_constructor,
                    CommandStatus::Failed(None),
                )
                .await?;
            let error = format!(
                "failed to spawn command: `{cmd_str}` with an error: {:#?}",
                err
            );
            error!(
                terrain_name = terrain_name,
                session_id = session_id,
                is_constructor = is_constructor,
                timestamp = timestamp,
                index = index,
                "{error}"
            );
            bail!(error);
        }
    }

    Ok(())
}
