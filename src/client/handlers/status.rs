use crate::client::args::History;
#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::common::types::pb;
use crate::common::types::pb::Error;
use crate::common::types::socket::Socket;
use crate::common::types::terrain_state::TerrainState;
use anyhow::{anyhow, Context as AnyhowContext, Result};
use prost_types::Any;

impl From<History> for i32 {
    fn from(value: History) -> Self {
        match value {
            History::Recent => pb::status_request::History::Recent as i32,
            History::Recent1 => pb::status_request::History::Recent1 as i32,
            History::Recent2 => pb::status_request::History::Recent2 as i32,
        }
    }
}

pub async fn handle(
    context: Context,
    json: bool,
    client: Option<Client>,
    history: History,
) -> Result<String> {
    if context.session_id().is_empty() && !context.toml_exists() {
        return Err(anyhow!(
            "terrain.toml does not exists, run `terrainium init` to initialize terrain."
        ));
    }

    let mut client = if let Some(client) = client {
        client
    } else {
        Client::new(context.socket_path())
            .await
            .context("failed to connect to daemon. check if `terrainiumd` is running")?
    };

    let terrain_name = context.name();

    let request = pb::StatusRequest {
        terrain_name,
        history: history.into(),
    };

    client
        .write_and_stop(Any::from_msg(&request).unwrap())
        .await?;

    let response: Any = client.read().await?;

    let status_response: Result<pb::StatusResponse> = response
        .to_msg()
        .context("failed to convert status response from any");

    if let Ok(status) = status_response {
        let status: TerrainState = status.into();
        Ok(status.rendered(json))
    } else {
        let error: Error = response
            .to_msg()
            .context("failed to convert to error from Any")?;

        Err(anyhow!(
            "error response from daemon {}",
            error.error_message
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::client::args::History;
    use crate::client::shell::Zsh;
    use crate::client::types::client::MockClient;
    use crate::client::types::context::Context;
    use crate::common::run::MockCommandToRun;
    use anyhow::anyhow;
    use std::path::PathBuf;

    #[tokio::test]
    async fn throws_error_if_no_terrainiumd_socket() {
        let terrainiumd_dir = tempfile::tempdir().unwrap();
        let socket_path = terrainiumd_dir.path().join("socket");
        let context = Context::build(
            PathBuf::new(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            socket_path.clone(),
        );

        let mock_client = MockClient::new_context();
        mock_client
            .expect()
            .withf(|_| true)
            .times(1)
            .return_once(move |_| {
                Err(anyhow!(
                    "Daemon Socket does not exist: {}",
                    socket_path.display()
                ))
            });

        let err = super::handle(context, false, None, History::Recent)
            .await
            .expect_err("Expected an error");

        assert_eq!(
            err.to_string(),
            "failed to connect to daemon. check if `terrainiumd` is running"
        );
    }

    #[tokio::test]
    async fn throws_error_if_no_active_terrain_outside_terrain_dir() {
        let terrainiumd_dir = tempfile::tempdir().unwrap();
        let current_dir = tempfile::tempdir().unwrap();
        let central_dir = tempfile::tempdir().unwrap();

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(MockCommandToRun::default()),
            terrainiumd_dir.path().join("socket"),
        );

        let err = super::handle(context, false, Some(MockClient::default()), History::Recent)
            .await
            .expect_err("Expected an error");

        assert_eq!(
            err.to_string(),
            "terrain.toml does not exists, run `terrainium init` to initialize terrain."
        );
    }
}
