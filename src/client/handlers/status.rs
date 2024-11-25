use crate::client::args::History;
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
) -> Result<()> {
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
        status.render(json).context("status to be rendered")?;
    } else {
        let error: Error = response
            .to_msg()
            .context("failed to convert to error from Any")?;

        return Err(anyhow!(
            "error response from daemon {}",
            error.error_message
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::client::args::History;
    use crate::client::shell::Zsh;
    use crate::client::types::context::Context;
    use crate::common::run::MockCommandToRun;
    use std::path::PathBuf;

    #[tokio::test]
    async fn throws_error_if_terrainiumd_is_not_running() {
        let terrainiumd_dir = tempfile::tempdir().unwrap();

        let context = Context::build(
            PathBuf::new(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
            terrainiumd_dir.path().join("socket"),
        );

        let err = super::handle(context, false, None, History::Recent)
            .await.expect_err("Expected an error");

        assert_eq!(err.to_string(), "failed to connect to daemon. check if `terrainiumd` is running");
    }
}
