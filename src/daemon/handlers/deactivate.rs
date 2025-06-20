use crate::common::types::pb;
use crate::common::types::pb::response::Payload::Body;
use crate::common::types::pb::{Deactivate, Response};
use crate::daemon::handlers::execute::spawn_commands;
use crate::daemon::handlers::{error_response, RequestHandler};
use crate::daemon::types::context::DaemonContext;
use anyhow::{Context, Result};
use prost_types::Any;
use std::sync::Arc;
use tracing::trace;

pub struct DeactivateHandler;
impl RequestHandler for DeactivateHandler {
    async fn handle(request: Any, context: Arc<DaemonContext>) -> Any {
        trace!("handling deactivate request");
        let request: Result<Deactivate> = request
            .to_msg()
            .context("failed to convert request to Deactivate");

        let response = match request {
            Ok(data) => deactivate(data, context).await,
            Err(err) => error_response(err),
        };
        Any::from_msg(&response).unwrap()
    }
}

async fn deactivate(request: Deactivate, context: Arc<DaemonContext>) -> Response {
    let Deactivate {
        session_id,
        terrain_name,
        end_timestamp,
        destructors,
    } = request;
    trace!(
        terrain_name = terrain_name,
        session_id = session_id,
        end_timestamp = end_timestamp,
        "executing deactivate request"
    );
    let mut result = context
        .state_manager()
        .update_end_time(&terrain_name, &session_id, end_timestamp)
        .await
        .context("failed to deactivate");

    if result.is_ok() {
        trace!(
            terrain_name = terrain_name,
            session_id = session_id,
            "updated end time successfully"
        );
        if let Some(destructors) = destructors {
            trace!("running destructors for deactivation request");
            result = spawn_commands(destructors, context).await;
        }
    }

    match result {
        Ok(()) => Response {
            payload: Some(Body(pb::Body { message: None })),
        },
        Err(err) => error_response(err),
    }
}

#[cfg(test)]
mod tests {
    use crate::client::types::terrain::AutoApply;
    use crate::common::constants::TERRAIN_STATE_FILE_NAME;
    use crate::common::execute::MockExecutor;
    use crate::common::test_utils::{
        expected_deactivate_request_example_biome, TEST_SESSION_ID, TEST_TERRAIN_NAME,
    };
    use crate::common::types::terrain_state::test_utils::{
        terrain_state_after_construct, terrain_state_after_deactivate_before_complete,
    };
    use crate::common::types::terrain_state::TerrainState;
    use crate::common::utils::{create_file, write_to_file};
    use crate::daemon::types::config::DaemonConfig;
    use crate::daemon::types::context::DaemonContext;
    use std::fs;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_deactivate() {
        let state_directory = tempdir().unwrap();
        let is_auto_apply = true;
        let auto_apply = AutoApply::All;

        let context = DaemonContext::new(
            false,
            DaemonConfig::default(),
            Arc::new(MockExecutor::new()),
            state_directory.path().to_str().unwrap(),
        )
        .await;

        // setup previous state with constructors already added
        let terrain_state_file = state_directory.path().join(format!(
            "{TEST_TERRAIN_NAME}/{TEST_SESSION_ID}/{TERRAIN_STATE_FILE_NAME}"
        ));
        let old_state =
            terrain_state_after_construct(TEST_SESSION_ID.to_string(), is_auto_apply, &auto_apply);

        let mut state_file = create_file(&terrain_state_file).await.unwrap();
        write_to_file(
            &mut state_file,
            serde_json::to_string_pretty(&old_state).unwrap(),
        )
        .await
        .unwrap();

        let request = expected_deactivate_request_example_biome(TEST_SESSION_ID);

        super::deactivate(request, Arc::new(context)).await;

        let actual_state: TerrainState =
            serde_json::from_str(&fs::read_to_string(&terrain_state_file).unwrap()).unwrap();
        assert_eq!(
            actual_state,
            terrain_state_after_deactivate_before_complete(
                state_directory.path().to_str().unwrap(),
                TEST_SESSION_ID.to_string(),
                is_auto_apply,
                &auto_apply
            )
        );
    }
}
