use crate::common::types::pb;
use crate::common::types::pb::response::Payload::Body;
use crate::common::types::pb::{Response, StatusRequest};
use crate::daemon::handlers::{error_response, RequestHandler};
use crate::daemon::types::context::DaemonContext;
use anyhow::{bail, Context, Result};
use prost_types::Any;
use std::sync::Arc;
use tracing::trace;

pub struct StatusHandler;

impl RequestHandler for StatusHandler {
    async fn handle(request: Any, context: Arc<DaemonContext>) -> Any {
        trace!("handling Status request");
        let request: Result<StatusRequest> = request
            .to_msg()
            .context("failed to convert request to Activate");

        let response = match request {
            Ok(data) => status(data, context)
                .await
                .context("failed to handle status request")
                .unwrap_or_else(error_response),
            Err(err) => error_response(err),
        };
        Any::from_msg(&response).unwrap()
    }
}

async fn status(request: StatusRequest, context: Arc<DaemonContext>) -> Result<Response> {
    let StatusRequest {
        identifier,
        terrain_name,
    } = request;

    if identifier.is_none() {
        bail!("identifier missing from status request");
    }

    let stored_history = context
        .state_manager()
        .get_or_create_history(&terrain_name)
        .await
        .context(format!("failed to create history file {terrain_name}"))?;

    let session_id = stored_history
        .read()
        .await
        .get_session(identifier.unwrap())
        .context("failed to get the session id from the history")?;

    let stored_state = context
        .state_manager()
        .refreshed_state(&terrain_name, &session_id)
        .await
        .context("failed to fetch the state")?;

    let state: pb::StatusResponse = stored_state.read().await.state().into();
    trace!(
        terrain_name = terrain_name,
        session_id = session_id,
        "successfully fetched state {state:#?}"
    );
    Ok(Response {
        payload: Some(Body(pb::Body {
            message: Some(state),
        })),
    })
}

#[cfg(test)]
mod tests {
    use crate::client::test_utils::expected_constructor_background_example_biome;
    use crate::client::types::terrain::AutoApply;
    use crate::common::constants::{
        EXAMPLE_BIOME, TERRAIN_HISTORY_FILE_NAME, TERRAIN_STATE_FILE_NAME, TERRAIN_TOML,
        TEST_TIMESTAMP,
    };
    use crate::common::test_utils::{
        expected_envs_with_activate_example_biome, expected_status_request, RequestFor,
        TEST_SESSION_ID, TEST_TERRAIN_DIR, TEST_TERRAIN_NAME, TEST_TIMESTAMP_NUMERIC,
    };
    use crate::common::types::paths::{get_terrainiumd_paths, DaemonPaths};
    use crate::common::types::pb::response::Payload;
    use crate::common::types::pb::status_response::{CommandState, CommandStates};
    use crate::common::types::pb::StatusResponse;
    use crate::common::types::terrain_state::test_utils::terrain_state_after_activate;
    use crate::common::utils::{create_file, write_to_file};
    use crate::daemon::handlers::status::status;
    use crate::daemon::types::context::DaemonContext;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;
    use tempfile::tempdir;

    fn expected_status_response(is_auto_apply: bool, auto_apply: &AutoApply) -> StatusResponse {
        let mut command_states = vec![];
        expected_constructor_background_example_biome(Path::new(TEST_TERRAIN_DIR))
            .into_iter()
            .enumerate()
            .for_each(|(idx, cmd)| {
                command_states.push(CommandState {
                    command: Some(cmd.into()),
                    log_path: format!("{}/{TEST_TERRAIN_NAME}/{TEST_SESSION_ID}/constructors.{idx}.{TEST_TIMESTAMP_NUMERIC}.log", get_terrainiumd_paths().dir_str()),
                    // status: 1 i.e. starting
                    status: 1,
                    // exit_code: -100 i.e. starting
                    exit_code: -100,
                });
            });

        let mut constructors = BTreeMap::new();
        constructors.insert(TEST_TIMESTAMP.to_string(), CommandStates { command_states });
        StatusResponse {
            session_id: TEST_SESSION_ID.to_string(),
            terrain_name: TEST_TERRAIN_NAME.to_string(),
            biome_name: EXAMPLE_BIOME.to_string(),
            terrain_dir: TEST_TERRAIN_DIR.to_string(),
            toml_path: Path::new(TEST_TERRAIN_DIR)
                .join(TERRAIN_TOML)
                .to_string_lossy()
                .to_string(),
            is_background: true,
            start_timestamp: TEST_TIMESTAMP.to_string(),
            end_timestamp: "".to_string(),
            envs: expected_envs_with_activate_example_biome(is_auto_apply, auto_apply),
            constructors,
            destructors: Default::default(),
        }
    }

    #[tokio::test]
    async fn test_status() {
        let state_directory = tempdir().unwrap();
        let state_dir_path = state_directory.path().to_path_buf();

        let terrain_dir_path = state_dir_path.join(TEST_TERRAIN_NAME);
        let session_dir_path = terrain_dir_path.join(TEST_SESSION_ID);
        let state_path = session_dir_path.join(TERRAIN_STATE_FILE_NAME);
        let history_path = terrain_dir_path.join(TERRAIN_HISTORY_FILE_NAME);

        fs::create_dir_all(&session_dir_path).unwrap();

        let is_auto_apply = true;
        let auto_apply = AutoApply::All;

        let old_state =
            terrain_state_after_activate(TEST_SESSION_ID.to_string(), is_auto_apply, &auto_apply);

        let mut state_file = create_file(&state_path).await.unwrap();
        write_to_file(
            &mut state_file,
            serde_json::to_string_pretty(&old_state).unwrap(),
        )
        .await
        .unwrap();

        let mut history_file = create_file(&history_path).await.unwrap();
        write_to_file(&mut history_file, format!("{TEST_SESSION_ID}\n\n\n\n"))
            .await
            .unwrap();

        let request = expected_status_request(RequestFor::None, TEST_SESSION_ID);
        let Payload::Body(response) = status(
            request,
            Arc::new(
                DaemonContext::new(
                    false,
                    Default::default(),
                    Default::default(),
                    Default::default(),
                    DaemonPaths::new(state_directory.path().to_str().unwrap()),
                )
                .await,
            ),
        )
        .await
        .unwrap()
        .payload
        .unwrap() else {
            panic!("unexpected status response");
        };
        let response = response.message.unwrap();

        assert_eq!(
            response,
            expected_status_response(is_auto_apply, &auto_apply)
        );
    }

    #[tokio::test]
    async fn test_status_from_history() {
        let state_directory = tempdir().unwrap();
        let state_dir_path = state_directory.path().to_path_buf();

        let terrain_dir_path = state_dir_path.join(TEST_TERRAIN_NAME);
        let session_dir_path = terrain_dir_path.join(TEST_SESSION_ID);
        let state_path = session_dir_path.join(TERRAIN_STATE_FILE_NAME);
        let history_path = terrain_dir_path.join(TERRAIN_HISTORY_FILE_NAME);

        fs::create_dir_all(&session_dir_path).unwrap();

        let is_auto_apply = true;
        let auto_apply = AutoApply::All;

        let old_state =
            terrain_state_after_activate(TEST_SESSION_ID.to_string(), is_auto_apply, &auto_apply);

        let mut state_file = create_file(&state_path).await.unwrap();
        write_to_file(
            &mut state_file,
            serde_json::to_string_pretty(&old_state).unwrap(),
        )
        .await
        .unwrap();

        let mut history_file = create_file(&history_path).await.unwrap();
        write_to_file(
            &mut history_file,
            format!("some-session-id\n{TEST_SESSION_ID}\n\n\n"),
        )
        .await
        .unwrap();

        let request = expected_status_request(RequestFor::Recent(1), TEST_SESSION_ID);
        let Payload::Body(response) = status(
            request,
            Arc::new(
                DaemonContext::new(
                    false,
                    Default::default(),
                    Default::default(),
                    Default::default(),
                    DaemonPaths::new(state_directory.path().to_str().unwrap()),
                )
                .await,
            ),
        )
        .await
        .unwrap()
        .payload
        .unwrap() else {
            panic!("unexpected status response");
        };
        let response = response.message.unwrap();

        assert_eq!(
            response,
            expected_status_response(is_auto_apply, &auto_apply)
        );
    }

    #[tokio::test]
    async fn throws_error_if_session_state_does_not_exist() {
        let state_directory = tempdir().unwrap();
        let state_dir_path = state_directory.path().to_path_buf();

        let terrain_dir_path = state_dir_path.join(TEST_TERRAIN_NAME);
        fs::create_dir_all(&terrain_dir_path).unwrap();

        let request = expected_status_request(RequestFor::None, "some-session-id");

        let error = status(
            request,
            Arc::new(
                DaemonContext::new(
                    false,
                    Default::default(),
                    Default::default(),
                    Default::default(),
                    DaemonPaths::new(state_directory.path().to_str().unwrap()),
                )
                .await,
            ),
        )
        .await
        .unwrap_err()
        .to_string();

        assert_eq!(error, "failed to fetch the state");
    }

    #[tokio::test]
    async fn throws_error_if_history_entry_does_not_exist() {
        let state_directory = tempdir().unwrap();
        let state_dir_path = state_directory.path().to_path_buf();

        let terrain_dir_path = state_dir_path.join(TEST_TERRAIN_NAME);
        let history_path = terrain_dir_path.join(TERRAIN_HISTORY_FILE_NAME);
        fs::create_dir_all(&terrain_dir_path).unwrap();

        let mut history_file = create_file(&history_path).await.unwrap();
        write_to_file(&mut history_file, format!("{TEST_SESSION_ID}\n\n\n\n"))
            .await
            .unwrap();

        let request = expected_status_request(RequestFor::Recent(2), "");

        let error = status(
            request,
            Arc::new(
                DaemonContext::new(
                    false,
                    Default::default(),
                    Default::default(),
                    Default::default(),
                    DaemonPaths::new(state_directory.path().to_str().unwrap()),
                )
                .await,
            ),
        )
        .await
        .unwrap_err()
        .to_string();

        assert_eq!(error, "failed to get the session id from the history");
    }

    #[tokio::test]
    async fn throws_error_if_recent_out_of_bounds() {
        let state_directory = tempdir().unwrap();
        let state_dir_path = state_directory.path().to_path_buf();

        let terrain_dir_path = state_dir_path.join(TEST_TERRAIN_NAME);
        let history_path = terrain_dir_path.join(TERRAIN_HISTORY_FILE_NAME);
        fs::create_dir_all(&terrain_dir_path).unwrap();

        let mut history_file = create_file(&history_path).await.unwrap();
        write_to_file(&mut history_file, format!("{TEST_SESSION_ID}\n\n\n\n"))
            .await
            .unwrap();

        // default size is 5
        let request = expected_status_request(RequestFor::Recent(10), "");

        let error = status(
            request,
            Arc::new(
                DaemonContext::new(
                    false,
                    Default::default(),
                    Default::default(),
                    Default::default(),
                    DaemonPaths::new(state_directory.path().to_str().unwrap()),
                )
                .await,
            ),
        )
        .await
        .unwrap_err()
        .to_string();

        assert_eq!(error, "failed to get the session id from the history");
    }
}
