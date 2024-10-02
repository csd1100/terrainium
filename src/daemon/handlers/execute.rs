use crate::common::constants::{CONSTRUCTORS, DESTRUCTORS, TERRAINIUMD_TMP_DIR};
#[double]
use crate::common::execute::Run;
use crate::common::types::pb;
use crate::common::types::pb::{ExecuteRequest, ExecuteResponse, Operation};
use crate::common::utils::timestamp;
use crate::daemon::handlers::RequestHandler;
use crate::daemon::types::terrain_state::{CommandStatus, TerrainState};
use anyhow::{Context, Result};
use mockall_double::double;
use prost_types::Any;
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;
use std::sync::Arc;
use tokio::fs;
use tokio::fs::create_dir_all;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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
                tokio::spawn(execute(request, None));
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
pub(crate) async fn execute(request: ExecuteRequest, state_file: Option<Arc<Mutex<fs::File>>>) {
    let terrain_name = request.terrain_name;

    let mut set = JoinSet::new();

    let commands = request.commands;
    let iter = commands.into_iter().enumerate();

    if state_file.is_none() {
        let terrain_dir = format!("{}/{}", TERRAINIUMD_TMP_DIR, terrain_name);
        event!(
            Level::DEBUG,
            "creating directory: {} for terrain: {} if not present",
            terrain_dir,
            terrain_name
        );
        create_dir_all(&terrain_dir.clone())
            .await
            .expect("create terrain dir");
    }

    for (idx, command) in iter {
        let (log_file, op) = if let Some(state_file) = state_file.as_ref() {
            let mut file = state_file.lock().await;
            let mut json: String = String::new();
            file.read_to_string(&mut json)
                .await
                .expect("failed to read state file");
            let state = TerrainState::from_json(json.as_str()).expect("failed to read state file");
            drop(file);

            let op = state.execute_request().operation().to_string();
            let log_file = format!("{}/{}.{}.log", state.terrain_name(), op, idx);
            (log_file, op)
        } else {
            let operation = Operation::try_from(request.operation).expect("invalid operation");
            let op = match operation {
                Operation::Unspecified => "unspecified",
                Operation::Constructors => CONSTRUCTORS,
                Operation::Destructors => DESTRUCTORS,
            }
            .to_string();
            let now = timestamp();
            let log_file = format!("{}/{}.{}.{}.log", terrain_name, op, idx, now);
            (log_file, op)
        };

        let file = if state_file.is_some() {
            Some(state_file.clone().unwrap())
        } else {
            None
        };

        let run: Run = Run::new(command.exe, command.args, Some(command.envs));
        event!(Level::INFO, "spawning operation: {:?}", op);
        set.spawn(async move {
            let process = format!("{:#?}", run);

            if file.is_some() {
                let file = file.clone().unwrap();
                let mut file = file.lock().await;
                let mut json: String = String::new();
                file.read_to_string(&mut json)
                    .await
                    .expect("failed to read state file");

                let mut state =
                    TerrainState::from_json(json.as_str()).expect("failed to read state file");

                state
                    .execute_request_mut()
                    .set_command_state(idx, CommandStatus::Running);
                state
                    .execute_request_mut()
                    .set_log_path(idx, log_file.clone());

                file.write_all(state.to_json().expect("to be serialized").as_bytes())
                    .await
                    .expect("failed to write state file");

                drop(file);
            }

            event!(
                Level::INFO,
                "operation:{}, starting process for command: {:?}",
                op,
                run
            );

            let res: Result<ExitStatus> = run.async_wait(&log_file).await;

            match res {
                Ok(exit_code) => {
                    event!(
                        Level::INFO,
                        "operation:{}, completed executing command with exit code: {}, process: {}",
                        op,
                        exit_code,
                        process
                    );

                    if file.is_some() {
                        let file = file.unwrap().clone();
                        let mut file = file.lock().await;
                        let mut json: String = String::new();
                        file.read_to_string(&mut json)
                            .await
                            .expect("failed to read state file");

                        let mut state = TerrainState::from_json(json.as_str())
                            .expect("failed to read state file");

                        if exit_code == ExitStatus::from_raw(0) {
                            state.execute_request_mut().set_command_state(
                                idx,
                                CommandStatus::Failed(exit_code.into_raw()),
                            );
                        } else {
                            state
                                .execute_request_mut()
                                .set_command_state(idx, CommandStatus::Succeeded);
                        }

                        file.write_all(state.to_json().expect("to be serialized").as_bytes())
                            .await
                            .expect("failed to write state file");

                        drop(file);
                    }
                }

                Err(err) => {
                    event!(
                        Level::WARN,
                        "operation:{}, failed to spawn command with error: {:?}, process:{}",
                        op,
                        err,
                        process
                    );
                }
            }
        });
    }
    let _results = set.join_all().await;
}
