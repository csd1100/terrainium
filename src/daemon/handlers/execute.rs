use crate::common::constants::TERRAINIUMD_TMP_DIR;
use crate::common::execute::Run;
use crate::common::types::pb;
use crate::common::types::pb::{ExecuteRequest, ExecuteResponse, Operation};
use crate::daemon::handlers::RequestHandler;
use anyhow::{Context, Result};
use prost_types::Any;
use tokio::fs;
use tokio::fs::create_dir_all;
use tokio::task::JoinSet;

pub(crate) struct ExecuteHandler;

impl RequestHandler for ExecuteHandler {
    async fn handle(request: Any) -> Any {
        let exe_request: Result<ExecuteRequest> = request
            .to_msg()
            .context("failed to convert request to type ExecuteRequest");

        match exe_request {
            Ok(request) => {
                println!("Received request: {:?}", request);
                tokio::spawn(execute(request));
                Any::from_msg(&ExecuteResponse {}).expect("to be converted to Any")
            }
            Err(err) => Any::from_msg(&pb::Error {
                error_message: err.to_string(),
            })
            .expect("to be converted to Any"),
        }
    }
}

async fn execute(request: ExecuteRequest) {
    let terrain_name = request.terrain_name;

    let mut set = JoinSet::new();

    let commands = request.commands;
    println!("Executing commands: {:?}", commands);
    let iter = commands.into_iter().enumerate();

    for (idx, command) in iter {
        let terrain_dir = format!("{}/{}", TERRAINIUMD_TMP_DIR, terrain_name);
        create_dir_all(&terrain_dir.clone())
            .await
            .expect("create terrain dir");

        let operation = Operation::from_i32(request.operation).expect("invalid operation");

        let op = match operation {
            Operation::Unspecified => "unspecified",
            Operation::Constructors => "constructors",
            Operation::Destructors => "destructors",
        }
        .to_string();
        let run: Run = command.into();
        set.spawn(async move {
            let log_file = fs::File::options()
                .create(true)
                .append(true)
                .open(format!("{}/{}.{}.log", terrain_dir, op, idx))
                .await
                .expect("failed to create / append to log file");

            let stdout: std::fs::File = log_file
                .try_clone()
                .await
                .expect("failed to clone file handle")
                .into_std()
                .await;

            let stderr: std::fs::File = log_file.into_std().await;

            run.async_wait(Some(stdout.into()), Some(stderr.into()))
                .await
                .expect("TODO: panic message");
        });
    }
    let _results = set.join_all().await;
}

#[cfg(test)]
mod tests {}
