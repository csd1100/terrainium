use anyhow::Result;

use crate::{
    proto::{
        self,
        command::{Args, CommandType},
        status_request::{History, Operation, ProcessRequest},
        Command, StatusRequest,
    },
    types::{args::Session, socket},
};

pub fn handle(
    session: Session,
    list_processes: bool,
    process_id: Option<u32>,
) -> Result<()> {
    let op = if list_processes {
        Some(Operation::ListBackground(true))
    } else {
        process_id.map(|id| {
            Operation::ProcessRequst(ProcessRequest {
                background_pid: id,
                no_of_std_lines: 7,
            })
        })
    };

    let request = StatusRequest {
        session: Some(session.into()),
        operation: op,
    };

    let mut socket = socket::Unix::new()?;
    socket.write(Command {
        r#type: CommandType::Status.into(),
        args: Some(Args::Status(request)),
    })?;
    Ok(())
}

impl From<Session> for proto::status_request::Session {
    fn from(val: Session) -> Self {
        match val {
            Session::Current(session_id) => proto::status_request::Session::SessionId(session_id),
            Session::Last => proto::status_request::Session::History(History::Last.into()),
            Session::Last1 => proto::status_request::Session::History(History::Last1.into()),
            Session::Last2 => proto::status_request::Session::History(History::Last2.into()),
        }
    }
}
