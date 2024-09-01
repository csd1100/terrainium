use core::panic;
use std::{
    fs::File,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use anyhow::Context;
use tracing::{event, span, Level, Span};

use super::{connection, listener::Listener};
use crate::{
    daemon::types::status::DaemonStatus,
    helpers::constants::{TERRAINIUMD_SOCK, TERRAINIUMD_STATUS_FILE},
};

pub fn handle(parent: &Span) {
    let start = span!(parent: parent, Level::TRACE, "start" );
    let _enter = start.enter();

    event!(Level::DEBUG, "creating new daemon status file");

    let daemon_status_file = File::create(PathBuf::from(TERRAINIUMD_STATUS_FILE));
    let daemon_status_file = match daemon_status_file {
        Ok(file) => file,
        Err(err) => {
            event!(
                Level::ERROR,
                "error while creating new daemon status file {:?}",
                err
            );
            panic!("error while creating new daemon status file {:?}", err)
        }
    };

    let res = serde_json::to_writer_pretty(&daemon_status_file, &DaemonStatus::new());
    if let Err(err) = res {
        event!(Level::ERROR, "error while writing status to file {:?}", err);
        panic!("error while writing status to file {:?}", err);
    }

    let daemon_status_file = File::options()
        .write(true)
        .read(true)
        .open(PathBuf::from(TERRAINIUMD_STATUS_FILE));

    if let Err(err) = daemon_status_file {
        event!(
            Level::ERROR,
            "error while opening daemon status file {:?}",
            err
        );
        panic!("error while opening daemon status file {:?}", err);
    }
    let daemon_status_file = Arc::new(Mutex::new(daemon_status_file.unwrap()));

    event!(Level::INFO, "starting unix daemon socket listener");
    let result =
        Listener::bind(TERRAINIUMD_SOCK).context(format!("unable to bind {}", TERRAINIUMD_SOCK));

    match result {
        Ok(listener) => {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        event!(Level::DEBUG, "successful connection");
                        let status_file = daemon_status_file.clone();
                        thread::spawn(|| {
                            let res = connection::handle(status_file, stream.into());
                            if let Err(err) = res {
                                event!(Level::ERROR, "error while handling the connection: {err}")
                            }
                        });
                    }
                    Err(err) => {
                        event!(Level::ERROR, "error while accepting connections: {err}")
                    }
                }
            }
        }
        Err(err) => {
            event!(
                Level::ERROR,
                "error while starting unix daemon socket listener: {:?}",
                err
            );
            panic!(
                "error while starting unix daemon socket listener: {:?}",
                err
            );
        }
    }
}
