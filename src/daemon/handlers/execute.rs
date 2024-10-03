#[double]
use crate::common::execute::Run;
use crate::common::types::pb;
use crate::common::types::pb::{ExecuteRequest, ExecuteResponse};
use crate::common::utils::timestamp;
use crate::daemon::handlers::activate::{get_state_dir_path, get_state_file, get_terrain_state};
use crate::daemon::handlers::RequestHandler;
use crate::daemon::types::terrain_state::{operation_name, CommandStatus, TerrainState};
use anyhow::{Context, Result};
use mockall_double::double;
use prost_types::Any;
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;
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
                tokio::spawn(execute(request, (false, timestamp())));
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
pub(crate) async fn execute(request: ExecuteRequest, activate_data: (bool, String)) {
    let (from_activate, timestamp) = activate_data;
    let terrain_state: TerrainState = request.clone().into();
    let op = operation_name(request.operation);
    let commands = request.commands;
    let iter = commands.into_iter().enumerate();

    let terrain_state = if !from_activate {
        if !fs::try_exists(get_state_dir_path(&request.terrain_name, &timestamp))
            .await
            .expect("failed to check if state dir exists")
        {
            fs::create_dir_all(get_state_dir_path(&request.terrain_name, &timestamp))
                .await
                .expect("failed to create state dir");
        }

        let mut file = get_state_file(&request.terrain_name, &timestamp, false, false, true).await;
        file.write_all(
            terrain_state
                .to_json()
                .expect("state to be parsed to json")
                .as_ref(),
        )
        .await
        .expect("failed to write to state file");

        terrain_state
    } else {
        get_terrain_state(&request.terrain_name, &timestamp).await
    };

    let terrain_name = request.terrain_name.clone();
    let arc = Arc::new(Mutex::new(terrain_state));
    let mut set = JoinSet::new();

    for (idx, command) in iter {
        let op = op.clone();
        let timestamp = timestamp.clone();
        let terrain_name = terrain_name.clone();

        let log_file = format!(
            "{}/{}.{}.{}.log",
            get_state_dir_path(&terrain_name, timestamp.as_str()),
            op,
            idx,
            timestamp
        );

        let state = arc.clone();
        {
            let mut guard = state.lock().await;
            guard
                .execute_request_mut()
                .set_log_path(idx, log_file.clone());

            get_state_file(&terrain_name, &timestamp, false, true, false)
                .await
                .write_all(
                    guard
                        .to_json()
                        .expect("state to be parsed to json")
                        .as_ref(),
                )
                .await
                .expect("Failed to write to state file");
        }

        let run: Run = Run::new(command.exe, command.args, Some(command.envs));

        event!(Level::INFO, "spawning operation: {:?}", op);

        let state = arc.clone();
        set.spawn(async move {
            event!(
                Level::INFO,
                "operation:{}, starting process for command: {:?}",
                op,
                run
            );
            start_process(idx, run, &log_file, state).await;
        });
    }
    let _results = set.join_all().await;
}

async fn start_process(idx: usize, run: Run, log_file: &str, state: Arc<Mutex<TerrainState>>) {
    {
        let mut guard = state.lock().await;
        guard
            .execute_request_mut()
            .set_command_state(idx, CommandStatus::Running);

        get_state_file(guard.terrain_name(), guard.timestamp(), false, true, false)
            .await
            .write_all(
                guard
                    .to_json()
                    .expect("state to be parsed to json")
                    .as_ref(),
            )
            .await
            .expect("Failed to write to state file");
    }

    let res: Result<ExitStatus> = run.async_wait(log_file).await;

    match res {
        Ok(exit_code) => {
            // event!(
            //     Level::INFO,
            //     "operation:{}, completed executing command with exit code: {}, process: {}",
            //     op,
            //     exit_code,
            //     process
            // );

            let mut guard = state.lock().await;

            if exit_code.success() {
                guard
                    .execute_request_mut()
                    .set_command_state(idx, CommandStatus::Succeeded);
            } else {
                guard
                    .execute_request_mut()
                    .set_command_state(idx, CommandStatus::Failed(exit_code.into_raw()));
            }

            get_state_file(guard.terrain_name(), guard.timestamp(), false, true, false)
                .await
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
            // event!(
            //     Level::WARN,
            //     "operation:{}, failed to spawn command with error: {:?}, process:{}",
            //     op,
            //     err,
            //     process
            // );

            let mut guard = state.lock().await;

            guard
                .execute_request_mut()
                .set_command_state(idx, CommandStatus::Failed(i32::MAX));

            get_state_file(guard.terrain_name(), guard.timestamp(), false, true, false)
                .await
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
