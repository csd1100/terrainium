use crate::common::constants::DESTRUCTORS;
use crate::common::execute::CommandToRun;
use crate::common::execute::Execute;
use crate::common::types::pb;
use crate::common::types::pb::{ExecuteRequest, ExecuteResponse};
use crate::daemon::handlers::RequestHandler;
use crate::daemon::types::context::DaemonContext;
use crate::daemon::types::terrain_state::{operation_name, CommandStatus, TerrainState};
use anyhow::{Context, Result};
use prost_types::Any;
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tracing::{event, instrument, Level};

pub(crate) struct ExecuteHandler;

impl RequestHandler for ExecuteHandler {
    #[instrument(skip(request))]
    async fn handle(request: Any, context: DaemonContext) -> Any {
        event!(Level::INFO, "handling ExecuteRequest");

        let exe_request: Result<ExecuteRequest> = request
            .to_msg()
            .context("failed to convert request to type ExecuteRequest");

        event!(
            Level::TRACE,
            "result of attempting to parse request: {:#?}",
            exe_request
        );

        match exe_request {
            Ok(request) => {
                event!(
                    Level::TRACE,
                    "spawning task to execute request {:#?}",
                    request
                );

                tokio::spawn(execute(request, context));

                Any::from_msg(&ExecuteResponse {}).expect("to be converted to Any")
            }
            Err(err) => {
                event!(Level::ERROR, "failed to parse the request {:#?}", err);
                Any::from_msg(&pb::Error {
                    error_message: err.to_string(),
                })
                .expect("to be converted to Any")
            }
        }
    }
}

#[instrument(skip(request))]
pub(crate) async fn execute(request: ExecuteRequest, context: DaemonContext) {
    let operation = operation_name(request.operation);
    let mut terrain_state: TerrainState = request.clone().into();

    if operation == DESTRUCTORS && !request.session_id.is_empty() {
        let mut json = String::new();
        terrain_state
            .readable_file()
            .await
            .expect("to be able to open file")
            .read_to_string(&mut json)
            .await
            .expect("to store file contents");

        let mut existing_terrain_state: TerrainState =
            TerrainState::from_json(&json).expect("to be able to parse");

        existing_terrain_state
            .merge(terrain_state)
            .expect("to be able to merge");
        terrain_state = existing_terrain_state;
    }

    let commands = terrain_state.execute_context().commands(&operation);
    let iter = commands.into_iter().enumerate();

    if !fs::try_exists(terrain_state.dir_path())
        .await
        .expect("failed to check if state dir exists")
    {
        fs::create_dir_all(terrain_state.dir_path())
            .await
            .expect("failed to create state dir");
    }

    terrain_state
        .new_file()
        .await
        .expect("to create new state file")
        .write_all(
            terrain_state
                .to_json()
                .expect("to convert state to json")
                .as_ref(),
        )
        .await
        .expect("to write state");

    let arc = Arc::new(Mutex::new(terrain_state));
    let mut set = JoinSet::new();

    for (idx, command) in iter {
        if command.get_exe().contains("sudo") && !context.should_run_sudo() {
            event!(
                Level::WARN,
                "not executing command for operation {operation} as running sudo is not allowed ({command})",
            );
            continue;
        }

        let operation = operation.clone();
        let state = arc.clone();
        {
            let mut guard = state.lock().await;
            guard.set_log_path(idx, &operation);
            guard
                .writable_file()
                .await
                .expect("to get state file")
                .write_all(guard.to_json().expect("to convert state to json").as_ref())
                .await
                .expect("to write state");

            event!(Level::INFO, "spawning operation: {operation}");
        }
        set.spawn(async move {
            start_process(idx, command, operation, state).await;
        });
    }
    let _results = set.join_all().await;
}

async fn start_process(
    idx: usize,
    command_to_run: CommandToRun,
    operation: String,
    state: Arc<Mutex<TerrainState>>,
) {
    let mut guard = state.lock().await;

    guard
        .execute_context_mut()
        .set_command_state(idx, &operation, CommandStatus::Running);

    guard
        .writable_file()
        .await
        .expect("to get state file")
        .write_all(
            guard
                .to_json()
                .expect("state to be parsed to json")
                .as_ref(),
        )
        .await
        .expect("Failed to write to state file");

    let log_file = guard
        .execute_context()
        .log_path(idx, &operation)
        .to_string();

    event!(
        Level::INFO,
        "operation:{operation}, starting to execute command with log_file: '{log_file}', process: '{}'",
        guard.execute_context().command(idx, &operation)
    );

    drop(guard);

    let res = command_to_run.async_wait(&log_file).await;

    match res {
        Ok(exit_code) => {
            let mut guard = state.lock().await;
            if exit_code.success() {
                event!(
                    Level::INFO,
                    "operation:{operation}, successfully completed executing command with exit code: {exit_code}, process: '{}'",
                    guard.execute_context().command(idx, &operation)
                );

                guard.execute_context_mut().set_command_state(
                    idx,
                    &operation,
                    CommandStatus::Succeeded,
                );
            } else {
                event!(
                    Level::WARN,
                    "operation:{operation}, completed executing command with exit code: {exit_code}, process: '{}'",
                    guard.execute_context().command(idx, &operation)
                );
                event!(
                    Level::DEBUG,
                    "operation:{operation}, failed process:{:?}",
                    guard.execute_context().command(idx, &operation)
                );

                guard.execute_context_mut().set_command_state(
                    idx,
                    &operation,
                    CommandStatus::Failed(exit_code.code()),
                );
            }

            guard
                .writable_file()
                .await
                .expect("to get state file")
                .write_all(
                    guard
                        .to_json()
                        .expect("state to be parsed to json")
                        .as_ref(),
                )
                .await
                .expect("Failed to write to state file");
        }

        Err(err) => {
            let mut guard = state.lock().await;
            event!(
                Level::WARN,
                "operation:{operation}, failed to spawn command with error: {err}, process:'{}'",
                guard.execute_context().command(idx, &operation)
            );
            event!(
                Level::DEBUG,
                "operation:{operation}, failed process:{:?}",
                guard.execute_context().command(idx, &operation)
            );

            guard.execute_context_mut().set_command_state(
                idx,
                &operation,
                CommandStatus::Failed(Some(i32::MAX)),
            );

            guard
                .writable_file()
                .await
                .expect("to get state file")
                .write_all(
                    guard
                        .to_json()
                        .expect("state to be parsed to json")
                        .as_ref(),
                )
                .await
                .expect("Failed to write to state file");
        }
    }
}
