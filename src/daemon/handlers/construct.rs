use crate::common::execute::{CommandToRun, Execute};
use crate::common::types::pb;
use crate::common::types::pb::response::Payload::Body;
use crate::common::types::pb::{Construct, Response};
use crate::common::types::terrain_state::{CommandState, CommandStatus, TerrainState};
use crate::common::utils::remove_non_numeric;
use crate::daemon::handlers::{error_response, RequestHandler};
use crate::daemon::types::context::DaemonContext;
use crate::daemon::types::state_manager::StoredState;
use anyhow::{bail, Context, Result};
use prost_types::Any;
use tracing::{debug, error, trace};

pub(crate) struct ConstructHandler;

impl RequestHandler for ConstructHandler {
    async fn handle(request: Any, context: DaemonContext) -> Any {
        trace!("handling Construct request");
        let activate: Result<Construct> = request
            .to_msg()
            .context("failed to convert request to Construct");

        trace!("result of attempting to parse request: {:#?}", activate);

        let response = match activate {
            Ok(construct) => {
                let result = spawn_constructors(construct, context)
                    .await
                    .context("failed to spawn constructors");
                if let Err(err) = result {
                    error_response(err)
                } else {
                    debug!("successfully spawned constructors");
                    Response {
                        payload: Some(Body(pb::Body { message: None })),
                    }
                }
            }
            Err(err) => error_response(err),
        };
        Any::from_msg(&response).unwrap()
    }
}

pub(crate) async fn spawn_constructors(
    constructors: Construct,
    context: DaemonContext,
) -> Result<()> {
    let timestamp = constructors.timestamp.clone();

    let (terrain_name, session_id) = if constructors.session_id.is_none() {
        // session_id is not provided that means running constructors outside
        // terrainium shell so create a new state

        let state: TerrainState = constructors.into();

        let terrain_name = state.terrain_name().to_string();
        let session_id = state.session_id().to_string();

        context.state_manager().create_state(state).await?;

        (terrain_name, session_id)
    } else {
        // if session_id is present check if CommandStatus is present for current
        // timestamp else add new entry
        let session_id = constructors.session_id.unwrap();
        let timestamp = constructors.timestamp;
        let numeric_timestamp = remove_non_numeric(&timestamp);
        let terrain_name = constructors.terrain_name;

        let commands = constructors
            .commands
            .into_iter()
            .enumerate()
            .map(|(index, cmd)| {
                CommandState::from(&terrain_name, &session_id, index, &numeric_timestamp, cmd)
            })
            .collect();

        context
            .state_manager()
            .add_commands_if_necessary(&terrain_name, &session_id, &timestamp, true, commands)
            .await
            .context("failed to add commands to state manager")?;

        (terrain_name, session_id)
    };

    let stored_state = context
        .state_manager()
        .refreshed_state(&terrain_name, &session_id)
        .await
        .context("failed to retrieve state from state manager")?;

    let commands = stored_state
        .clone()
        .read()
        .await
        .commands(true, &timestamp)?;

    commands
        .into_iter()
        .enumerate()
        .for_each(|(index, cmd_state)| {
            let stored_state = stored_state.clone();
            let timestamp = timestamp.clone();
            let CommandState {
                command, log_path, ..
            } = cmd_state;
            tokio::spawn(async move {
                let res =
                    spawn_command(stored_state, true, timestamp, index, command, log_path).await;

                if let Err(err) = res {
                    error!("failed to spawn command: {:?}", err);
                }
            });
        });

    Ok(())
}

async fn spawn_command(
    stored_state: StoredState,
    is_constructor: bool,
    timestamp: String,
    index: usize,
    command: CommandToRun,
    log_path: String,
) -> Result<()> {
    let cmd_str = command.to_string();
    let state = stored_state.read().await;
    let terrain_name = state.terrain_name().to_string();
    let session_id = state.session_id().to_string();
    // drop state to relieve read lock
    drop(state);

    debug!(
        terrain_name = terrain_name,
        session_id = session_id,
        is_constructor = is_constructor,
        timestamp = timestamp,
        index = index,
        "running command {cmd_str}"
    );

    let mut state_mut = stored_state.write().await;
    state_mut
        .update_command_status(is_constructor, &timestamp, index, CommandStatus::Running)
        .await?;
    drop(state_mut);

    let res = command.async_wait(&log_path).await;

    let mut state_mut = stored_state.write().await;
    match res {
        Ok(exit_status) => {
            if exit_status.success() {
                state_mut
                    .update_command_status(
                        is_constructor,
                        &timestamp,
                        index,
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
                state_mut
                    .update_command_status(
                        is_constructor,
                        &timestamp,
                        index,
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
            state_mut
                .update_command_status(
                    is_constructor,
                    &timestamp,
                    index,
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
