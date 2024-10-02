use crate::common::constants::{CONSTRUCTORS, DESTRUCTORS, TERRAINIUMD_TMP_DIR};
#[double]
use crate::common::execute::Run;
use crate::common::types::pb;
use crate::common::types::pb::{ExecuteRequest, ExecuteResponse, Operation};
use crate::daemon::handlers::RequestHandler;
use anyhow::{Context, Result};
use mockall_double::double;
use prost_types::Any;
use tokio::fs::create_dir_all;
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
async fn execute(request: ExecuteRequest) {
    let terrain_name = request.terrain_name;

    let mut set = JoinSet::new();

    let commands = request.commands;
    let iter = commands.into_iter().enumerate();

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

    for (idx, command) in iter {
        let terrain_dir = format!("{}/{}", TERRAINIUMD_TMP_DIR, terrain_name);
        let operation = Operation::try_from(request.operation).expect("invalid operation");

        let op = match operation {
            Operation::Unspecified => "unspecified",
            Operation::Constructors => CONSTRUCTORS,
            Operation::Destructors => DESTRUCTORS,
        }
        .to_string();

        let run: Run = Run::new(command.exe, command.args, Some(command.envs));

        let now = timestamp();

        event!(Level::INFO, "spawning operation: {:?}", op);
        set.spawn(async move {
            let process = format!("{:#?}", run);

            event!(
                Level::INFO,
                "operation:{}, starting process for command: {:?}",
                op,
                run
            );
            let res = run
                .async_wait(&format!("{}/{}.{}.{}.log", terrain_dir, op, idx, now))
                .await;

            match res {
                Ok(exit_code) => {
                    event!(
                        Level::INFO,
                        "operation:{}, completed executing command with exit code: {}, process: {}",
                        op,
                        exit_code,
                        process
                    );
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

fn timestamp() -> String {
    if let Ok(now) = time::OffsetDateTime::now_local() {
        now.format(
            &time::format_description::parse("[year]-[month]-[day]_[hour]:[minute]:[second]")
                .expect("time format to be parsed"),
        )
    } else {
        time::OffsetDateTime::now_utc().format(
            &time::format_description::parse("[year]-[month]-[day]_[hour]:[minute]:[second]")
                .expect("time format to be parsed"),
        )
    }
    .expect("time to be formatted")
}
