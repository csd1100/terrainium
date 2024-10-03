use crate::common::execute::CommandToRun;
use crate::common::execute::Execute;
use crate::common::types::pb;
use crate::common::types::pb::{ExecuteRequest, ExecuteResponse};
use crate::daemon::handlers::RequestHandler;
use crate::daemon::types::terrain_state::{CommandStatus, TerrainState};
use anyhow::{Context, Result};
use prost_types::Any;
use std::os::unix::process::ExitStatusExt;
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tracing::{event, instrument, Level};

pub(crate) struct ExecuteHandler;

impl RequestHandler for ExecuteHandler {
    #[instrument(skip(request))]
    async fn handle(request: Any) -> Any {
        event!(Level::INFO, "handling ExecuteRequest");

        let exe_request: Result<ExecuteRequest> = request
            .to_msg()
            .context("failed to convert request to type ExecuteRequest");

        event!(
            Level::DEBUG,
            "result of attempting to parse request: {:#?}",
            exe_request
        );

        match exe_request {
            Ok(request) => {
                event!(
                    Level::DEBUG,
                    "spawning task to execute request {:#?}",
                    request
                );

                tokio::spawn(execute(request));

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
pub(crate) async fn execute(request: ExecuteRequest) {
    let terrain_state: TerrainState = request.clone().into();

    let commands = terrain_state.execute_context().commands();
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
        let state = arc.clone();
        {
            let mut guard = state.lock().await;
            guard.set_log_path(idx);
            guard
                .writable_file()
                .await
                .expect("to get state file")
                .write_all(guard.to_json().expect("to convert state to json").as_ref())
                .await
                .expect("to write state");

            event!(
                Level::INFO,
                "spawning operation: {:?}",
                guard.execute_context().operation()
            );
        }
        set.spawn(async move {
            start_process(idx, command, state).await;
        });
    }
    let _results = set.join_all().await;
}

async fn start_process(idx: usize, command_to_run: CommandToRun, state: Arc<Mutex<TerrainState>>) {
    let mut guard = state.lock().await;

    guard
        .execute_context_mut()
        .set_command_state(idx, CommandStatus::Running);

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

    let log_file = guard.execute_context().log_path(idx).to_string();

    event!(
        Level::INFO,
        "operation:{}, starting to execute command with log_file: {}, process: {:?}",
        guard.execute_context().operation(),
        log_file,
        guard.execute_context().command(idx)
    );

    drop(guard);

    let res = command_to_run.async_wait(&log_file).await;

    match res {
        Ok(exit_code) => {
            let mut guard = state.lock().await;
            event!(
                Level::INFO,
                "operation:{}, completed executing command with exit code: {}, process: {:?}",
                guard.execute_context().operation(),
                exit_code,
                guard.execute_context().command(idx)
            );

            if exit_code.success() {
                guard
                    .execute_context_mut()
                    .set_command_state(idx, CommandStatus::Succeeded);
            } else {
                guard
                    .execute_context_mut()
                    .set_command_state(idx, CommandStatus::Failed(exit_code.into_raw()));
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
                "operation:{}, failed to spawn command with error: {:?}, process:{:?}",
                guard.execute_context().operation(),
                err,
                guard.execute_context().command(idx)
            );

            guard
                .execute_context_mut()
                .set_command_state(idx, CommandStatus::Failed(i32::MAX));

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
