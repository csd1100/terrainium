use anyhow::Result;

use crate::{
    proto,
    types::{args::Session, socket},
};

pub fn handle(session: Session, list_processes: bool, process_id: Option<u32>) -> Result<()> {
    let op: Option<proto::status_request::Operation> = if list_processes {
        Some(proto::status_request::Operation::ListBackground(true))
    } else {
        process_id.map(proto::status_request::Operation::BackgroundPid)
    };

    let status_request = proto::StatusRequest { operation: op };

    let mut socket = socket::Unix::new()?;
    socket.write(proto::Request {
        session: Some(session.into()),
        args: Some(proto::request::Args::Status(status_request)),
    })?;

    Ok(())
}

impl From<Session> for proto::request::Session {
    fn from(val: Session) -> Self {
        match val {
            Session::Current(session_id) => proto::request::Session::SessionId(session_id),
            Session::Last => proto::request::Session::History(proto::request::History::Last.into()),
            Session::Last1 => {
                proto::request::Session::History(proto::request::History::Last1.into())
            }
            Session::Last2 => {
                proto::request::Session::History(proto::request::History::Last2.into())
            }
        }
    }
}
