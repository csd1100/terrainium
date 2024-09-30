use crate::common::types::socket::Socket;
use crate::daemon::handlers::execute::ExecuteHandler;
#[double]
use crate::daemon::types::daemon_socket::DaemonSocket;
use anyhow::Result;
use mockall_double::double;
use prost_types::Any;
use tracing::instrument;

pub mod execute;

pub(crate) trait RequestHandler {
    async fn handle(request: Any) -> Any;
}

#[instrument]
pub async fn handle_request(mut daemon_socket: DaemonSocket) {
    let data: Result<Any> = daemon_socket.read().await;
    let response = match data {
        Ok(request) => match request.type_url.as_str() {
            "/terrainium.v1.ExecuteRequest" => ExecuteHandler::handle(request).await,
            "/terrainium.v1.ActivateRequest" => {
                todo!()
            }
            "/terrainium.v1.StatusRequest" => {
                todo!()
            }
            _ => panic!("invalid request type"),
        },
        Err(_) => todo!(),
    };

    let result = daemon_socket.write_and_stop(response).await;

    if result.is_err() {
        eprintln!("error responding execute request: {:?}", result);
    }
}

#[cfg(test)]
mod tests {
    use crate::common::types::pb::{Command, ExecuteRequest, ExecuteResponse, Operation};
    use crate::daemon::types::daemon_socket::MockDaemonSocket;
    use prost_types::Any;
    use std::collections::BTreeMap;

    #[tokio::test]
    async fn handle_execute() {
        let mut mocket = MockDaemonSocket::default();
        mocket
            .expect_read()
            .with()
            .return_once(move || {
                let mut envs: BTreeMap<String, String> = BTreeMap::new();
                envs.insert("EDITOR".to_string(), "nvim".to_string());
                envs.insert("PAGER".to_string(), "less".to_string());
                let expected = ExecuteRequest {
                    terrain_name: "terrainium".to_string(),
                    operation: i32::from(Operation::Constructors),
                    commands: vec![Command {
                        exe: "/bin/bash".to_string(),
                        args: vec![
                            "-c".to_string(),
                            "$PWD/tests/scripts/print_num_for_10_sec".to_string(),
                        ],
                        envs,
                    }],
                };
                Ok(Any::from_msg(&expected).expect("to be converted to any"))
            })
            .times(1);

        mocket
            .expect_write_and_stop()
            .withf(|actual| {
                actual == &Any::from_msg(&ExecuteResponse {}).expect("to be converted to any")
            })
            .times(1)
            .return_once(|_| Ok(()));

        super::handle_request(mocket).await;
    }
}
