use std::{
    fs::File,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, Context, Result};
use prost::Message;
use tracing::{event, span, Level};

use crate::{
    daemon::types::client_status::session_id_from, helpers::utils::fs::create_dir_if_not_exist,
    proto, types::socket,
};

use super::{
    handlers::activate,
    types::status::{status_from, DaemonStatus},
};

fn get_status_file_for(
    session_id: String,
    daemon_status_file: Arc<Mutex<File>>,
) -> Result<Arc<Mutex<File>>> {
    let daemon_status: DaemonStatus = status_from(daemon_status_file)?;

    if let Some(session) = daemon_status.active_terrains.get(&session_id) {
        let mut tmp_dir = PathBuf::from(format!(
            "/tmp/terrain-{}-{}-{}",
            session.name.clone(),
            session.biome.clone(),
            session_id.clone()
        ));

        event!(
            Level::INFO,
            "creating a temp directory for {}, {:?}",
            session_id,
            &tmp_dir
        );
        create_dir_if_not_exist(&tmp_dir)
            .context("failed to create terrain temporary directory")?;
        tmp_dir.push("status.json");

        let status_file = File::options()
            .write(true)
            .open(tmp_dir)
            .context("error opening terrain status file")?;

        Ok(Arc::new(Mutex::new(status_file)))
    } else {
        Err(anyhow!(format!(
            "session {} not found in active terrains",
            session_id
        )))
    }
}

pub fn handle(daemon_status_file: Arc<Mutex<File>>, mut socket: socket::Unix) -> Result<()> {
    let handle = span!(Level::TRACE, "connection.handle");
    let _enter = handle.enter();

    event!(Level::DEBUG, "parsing a message");
    let request: proto::Request =
        proto::Request::decode(socket.read()?).context("unable to parse request")?;

    let session_id: String = session_id_from(&request)?;

    let handle = span!(target: "connection.handle", Level::TRACE, "session", id = session_id );
    let _enter = handle.enter();

    let _status_file = if let Some(args) = &request.args {
        match args {
            proto::request::Args::Activate(_) => None,
            _ => {
                event!(Level::DEBUG, "getting session status file");
                Some(
                    get_status_file_for(session_id.clone(), daemon_status_file.clone()).context(
                        format!(
                            "error while getting session status file for session {}",
                            session_id.clone()
                        ),
                    )?,
                )
            }
        }
    } else {
        None
    };

    event!(Level::DEBUG, "parsing a arguments");
    let args = request.args;
    let result: Result<Option<proto::response::success::Body>, anyhow::Error> = args
        .map(|args| match args {
            proto::request::Args::Activate(request) => activate::handle(
                &handle,
                session_id.clone(),
                daemon_status_file.clone(),
                request,
            )
            .map(proto::response::success::Body::Activate),
            proto::request::Args::Execute(_) => todo!(),
            proto::request::Args::Status(_) => todo!(),
        })
        .transpose();

    let response: proto::Response = match result {
        Ok(data) => match data {
            Some(body) => {
                event!(Level::DEBUG, "writing response to client {:?}", body);
                proto::Response {
                    result: Some(proto::response::Result::Success(proto::response::Success {
                        body: Some(body),
                    })),
                }
            }
            None => {
                event!(Level::ERROR, "no response body was generated");
                proto::Response {
                    result: Some(proto::response::Result::Error(proto::response::Error {
                        error_message: "no response body was generated".to_string(),
                    })),
                }
            }
        },
        Err(err) => {
            event!(Level::ERROR, "error while generating response: {:?}", err);
            proto::Response {
                result: Some(proto::response::Result::Error(proto::response::Error {
                    error_message: format!("error while generating response: {}", err),
                })),
            }
        }
    };

    socket.write(response).context(format!(
        "error writing response to client for session {}:",
        session_id
    ))?;

    Ok(())
}
