use crate::common::types::pb;
use crate::common::types::pb::response::Payload::Body;
use crate::common::types::pb::{Activate, Response};
use crate::common::types::terrain_state::TerrainState;
use crate::daemon::handlers::execute::spawn_commands;
use crate::daemon::handlers::{error_response, RequestHandler};
use crate::daemon::types::context::DaemonContext;
use anyhow::{Context, Result};
use prost_types::Any;
use std::sync::Arc;
use tracing::trace;

pub(crate) struct ActivateHandler;

impl RequestHandler for ActivateHandler {
    async fn handle(request: Any, context: Arc<DaemonContext>) -> Any {
        trace!("handling Activate request");
        let request: Result<Activate> = request
            .to_msg()
            .context("failed to convert request to Activate");

        let response = match request {
            Ok(data) => activate(data, context)
                .await
                .context("failed to activate")
                .unwrap_or_else(error_response),
            Err(err) => error_response(err),
        };
        Any::from_msg(&response).unwrap()
    }
}

async fn activate(request: Activate, context: Arc<DaemonContext>) -> Result<Response> {
    let terrain_name = request.terrain_name.to_string();
    let session_id = request.session_id.to_string();
    trace!(
        terrain_name = terrain_name,
        session_id = session_id,
        "starting activation of terrain"
    );

    let constructors = request.constructors.clone();
    create_state(request, &context)
        .await
        .context("failed to create state while activating")?;

    trace!(
        terrain_name = terrain_name,
        session_id = session_id,
        "successfully created state"
    );
    if let Some(constructors) = constructors {
        trace!(
            terrain_name = terrain_name,
            session_id = session_id,
            "spawning constructors for activation request"
        );
        spawn_commands(constructors, context)
            .await
            .context("failed to spawn constructors while activating")?;
    }

    Ok(Response {
        payload: Some(Body(pb::Body { message: None })),
    })
}

async fn create_state(request: Activate, context: &DaemonContext) -> Result<()> {
    trace!("creating state for {request:#?}");
    let state: TerrainState = request.into();
    context.state_manager().create_state(state).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::client::types::terrain::AutoApply;
    use crate::common::constants::{TERRAIN_HISTORY_FILE_NAME, TERRAIN_STATE_FILE_NAME};
    use crate::common::execute::MockExecutor;
    use crate::common::test_utils::{
        expected_activate_request_example_biome, TEST_SESSION_ID, TEST_TERRAIN_NAME,
    };
    use crate::common::types::terrain_state::test_utils::terrain_state_after_activate;
    use crate::common::types::terrain_state::TerrainState;
    use crate::common::utils::{create_file, write_to_file};
    use crate::daemon::types::config::DaemonConfig;
    use crate::daemon::types::context::DaemonContext;
    use std::fs;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_state_on_activate() {
        let state_directory = tempdir().unwrap();
        let state_dir_path = state_directory.path().to_string_lossy().to_string();
        let terrain_state_dir = state_directory
            .path()
            .join(TEST_TERRAIN_NAME)
            .join(TEST_SESSION_ID);
        let terrain_state = terrain_state_dir.join(TERRAIN_STATE_FILE_NAME);

        let history = state_directory
            .path()
            .join(TEST_TERRAIN_NAME)
            .join(TERRAIN_HISTORY_FILE_NAME);

        let context = Arc::new(
            DaemonContext::new(
                false,
                DaemonConfig::default(),
                Arc::new(MockExecutor::default()),
                Default::default(),
                &state_dir_path,
            )
            .await,
        );

        let is_auto_apply = true;
        let auto_apply = AutoApply::All;

        let expected_request =
            expected_activate_request_example_biome(true, is_auto_apply, &auto_apply);

        super::activate(expected_request, context).await.unwrap();

        assert!(terrain_state.exists());
        assert!(history.exists());

        let history_contents = fs::read_to_string(&history).unwrap();
        assert_eq!(history_contents, format!("{TEST_SESSION_ID}\n\n\n\n"));

        let actual_state: TerrainState =
            serde_json::from_str(&fs::read_to_string(&terrain_state).unwrap()).unwrap();
        assert_eq!(
            actual_state,
            terrain_state_after_activate(TEST_SESSION_ID.to_string(), is_auto_apply, &auto_apply)
        );
    }

    #[tokio::test]
    async fn test_create_state_history_rotates() {
        let state_directory = tempdir().unwrap();
        let state_dir_path = state_directory.path().to_string_lossy().to_string();
        let terrain_state_dir = state_directory
            .path()
            .join(TEST_TERRAIN_NAME)
            .join(TEST_SESSION_ID);
        let terrain_state = terrain_state_dir.join(TERRAIN_STATE_FILE_NAME);

        let history = state_directory
            .path()
            .join(TEST_TERRAIN_NAME)
            .join(TERRAIN_HISTORY_FILE_NAME);

        let context = Arc::new(
            DaemonContext::new(
                false,
                DaemonConfig::default(),
                Arc::new(MockExecutor::default()),
                Default::default(),
                &state_dir_path,
            )
            .await,
        );

        let is_auto_apply = true;
        let auto_apply = AutoApply::All;

        let expected_request =
            expected_activate_request_example_biome(true, is_auto_apply, &auto_apply);

        let mut history_file = create_file(&history).await.unwrap();
        write_to_file(&mut history_file, format!("{TEST_SESSION_ID}-1\n{TEST_SESSION_ID}-2\n{TEST_SESSION_ID}-3\n{TEST_SESSION_ID}-4\n{TEST_SESSION_ID}-5")).await.unwrap();

        super::activate(expected_request, context).await.unwrap();

        assert!(terrain_state.exists());

        let history_contents = fs::read_to_string(&history).unwrap();
        assert_eq!(history_contents, format!("{TEST_SESSION_ID}\n{TEST_SESSION_ID}-1\n{TEST_SESSION_ID}-2\n{TEST_SESSION_ID}-3\n{TEST_SESSION_ID}-4"));

        let actual_state: TerrainState =
            serde_json::from_str(&fs::read_to_string(&terrain_state).unwrap()).unwrap();
        assert_eq!(
            actual_state,
            terrain_state_after_activate(TEST_SESSION_ID.to_string(), is_auto_apply, &auto_apply)
        );
    }
}
