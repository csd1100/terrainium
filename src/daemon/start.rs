use std::thread;

use anyhow::Context;
use tracing::{event, span, Level, Span};

use super::{connection, listener::Listener};
use crate::helpers::constants::TERRAINIUMD_SOCK;

pub fn handle(parent: &Span) {
    let start = span!(parent: parent, Level::TRACE, "start" );
    let _enter = start.enter();

    event!(Level::INFO, "starting unix daemon socket listener");
    let result =
        Listener::bind(TERRAINIUMD_SOCK).context(format!("unable to bind {}", TERRAINIUMD_SOCK));

    match result {
        Ok(listener) => {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        event!(Level::DEBUG, "successful connection");
                        thread::spawn(|| {
                            let res = connection::handle(stream);
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
            )
        }
    }
}
