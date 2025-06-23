use crate::common::execute::Execute;
#[mockall_double::double]
use crate::common::execute::Executor;
use crate::common::types::command::Command;
use crate::common::types::pb;
use crate::common::types::pb::response::Payload::Body;
use crate::common::types::pb::Response;
use crate::common::types::terrain_state::{CommandState, CommandStatus, TerrainState};
use crate::common::utils::remove_non_numeric;
use crate::daemon::handlers::{error_response, RequestHandler};
use crate::daemon::types::context::DaemonContext;
use crate::daemon::types::state_manager::{StoredHistory, StoredState};
use anyhow::{bail, Context, Result};
use prost_types::Any;
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::{debug, error, trace};

pub(crate) struct ExecuteHandler;

impl RequestHandler for ExecuteHandler {
    async fn handle(request: Any, context: Arc<DaemonContext>) -> Any {
        trace!("handling Execute request");
        let execute: Result<pb::Execute> = request
            .to_msg()
            .context("failed to convert request to Execute");

        let response = match execute {
            Ok(commands) => {
                let result = spawn_commands(commands, context)
                    .await
                    .context("failed to spawn commands");
                if let Err(err) = result {
                    error_response(err)
                } else {
                    debug!("successfully spawned commands");
                    Response {
                        payload: Some(Body(pb::Body { message: None })),
                    }
                }
            }
            Err(err) => error_response(err),
        };
        Any::from_msg(&response).unwrap()
    }
}

struct CommandInfo {
    index: usize,
    command: Command,
    envs: Arc<BTreeMap<String, String>>,
    is_constructor: bool,
    timestamp: String,
    log_path: String,
}

pub(crate) async fn spawn_commands(
    request: pb::Execute,
    context: Arc<DaemonContext>,
) -> Result<()> {
    let timestamp = request.timestamp.clone();
    let is_constructor = request.is_constructor;

    let (terrain_name, session_id) = if request.session_id.is_none() {
        // session_id is not provided that means running constructors or destructors
        // outside terrainium shell so create a new state

        let state: TerrainState = request.into();

        let terrain_name = state.terrain_name().to_string();
        let session_id = state.session_id().to_string();

        debug!(
            terrain_name = terrain_name,
            timestamp = timestamp,
            is_constructor = is_constructor,
            "execute request does not have associated session id"
        );
        context.state_manager().create_state(state).await?;

        (terrain_name, session_id)
    } else {
        // if session_id is present check if CommandStatus is present for current
        // timestamp else add new entry
        let session_id = request.session_id.unwrap();
        let numeric_timestamp = remove_non_numeric(&timestamp);
        let terrain_name = request.terrain_name;

        let commands = request
            .commands
            .into_iter()
            .enumerate()
            .map(|(index, cmd)| {
                CommandState::from(
                    context.state_paths().dir_str(),
                    &terrain_name,
                    &session_id,
                    is_constructor,
                    index,
                    &numeric_timestamp,
                    cmd,
                )
            })
            .collect();

        context
            .state_manager()
            .add_commands_if_necessary(
                &terrain_name,
                &session_id,
                &timestamp,
                is_constructor,
                commands,
            )
            .await
            .context("failed to add commands to state manager")?;

        (terrain_name, session_id)
    };

    let stored_state = context
        .state_manager()
        .refreshed_state(&terrain_name, &session_id)
        .await
        .context("failed to retrieve state from state manager")?;

    let envs = Arc::new(stored_state.clone().read().await.envs());
    let commands = stored_state
        .clone()
        .read()
        .await
        .commands(is_constructor, &timestamp)?;

    let history = context
        .state_manager()
        .get_or_create_history(&terrain_name)
        .await
        .context(format!("failed to create history file {terrain_name}"))?;

    commands
        .into_iter()
        .enumerate()
        .for_each(|(index, cmd_state)| {
            let history = history.clone();
            let stored_state = stored_state.clone();
            let timestamp = timestamp.clone();
            let executor = context.executor();
            let envs = envs.clone();
            let (command, log_path) = cmd_state.command_and_log_path();
            tokio::spawn(async move {
                let res = spawn_command(
                    executor,
                    history,
                    stored_state,
                    CommandInfo {
                        index,
                        command,
                        envs,
                        is_constructor,
                        timestamp,
                        log_path,
                    },
                )
                .await;

                if let Err(err) = res {
                    error!("failed to spawn command: {:?}", err);
                }
            });
        });

    Ok(())
}

async fn spawn_command(
    executor: Arc<Executor>,
    history: StoredHistory,
    stored_state: StoredState,
    command_info: CommandInfo,
) -> Result<()> {
    let CommandInfo {
        index,
        command,
        envs,
        is_constructor,
        timestamp,
        log_path,
    } = command_info;

    let cmd_str = command.to_string();
    let state = stored_state.read().await;
    let terrain_name = state.terrain_name().to_string();
    let session_id = state.session_id().to_string();
    // drop state to relieve read lock
    drop(state);

    debug!(
        terrain_name = terrain_name,
        session_id = session_id,
        is_constructor = is_constructor,
        timestamp = timestamp,
        index = index,
        "running command {cmd_str}"
    );

    let mut state_mut = stored_state.write().await;
    state_mut
        .update_command_status(
            history.clone(),
            is_constructor,
            &timestamp,
            index,
            CommandStatus::Running,
        )
        .await?;
    drop(state_mut);

    let res = executor
        .async_spawn_with_log(&log_path, Some(envs), command)
        .await;

    let mut state_mut = stored_state.write().await;
    match res {
        Ok(exit_status) => {
            if exit_status.success() {
                state_mut
                    .update_command_status(
                        history,
                        is_constructor,
                        &timestamp,
                        index,
                        CommandStatus::Succeeded,
                    )
                    .await?;
                debug!(
                    terrain_name = terrain_name,
                    session_id = session_id,
                    is_constructor = is_constructor,
                    timestamp = timestamp,
                    index = index,
                    "command {cmd_str} completed successfully"
                );
            } else {
                state_mut
                    .update_command_status(
                        history,
                        is_constructor,
                        &timestamp,
                        index,
                        CommandStatus::Failed(exit_status.code()),
                    )
                    .await?;

                let error = format!(
                    "command: {cmd_str} exited with code {:?}",
                    exit_status.code()
                );
                error!(
                    terrain_name = terrain_name,
                    session_id = session_id,
                    is_constructor = is_constructor,
                    timestamp = timestamp,
                    index = index,
                    "{error}"
                );
                bail!(error);
            }
        }
        Err(err) => {
            state_mut
                .update_command_status(
                    history,
                    is_constructor,
                    &timestamp,
                    index,
                    CommandStatus::Failed(None),
                )
                .await?;
            let error = format!(
                "failed to spawn command: {cmd_str} with an error: {:#?}",
                err
            );
            error!(
                terrain_name = terrain_name,
                session_id = session_id,
                is_constructor = is_constructor,
                timestamp = timestamp,
                index = index,
                "{error}"
            );
            bail!(error);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::client::test_utils::assertions::executor::{AssertExecutor, ExpectedCommand};
    use crate::client::test_utils::expected_env_vars_example_biome;
    use crate::client::types::terrain::AutoApply;
    use crate::common::constants::{
        TERRAIN_HISTORY_FILE_NAME, TERRAIN_STATE_FILE_NAME, TEST_TIMESTAMP,
    };
    use crate::common::execute::MockExecutor;
    use crate::common::test_utils::{
        expected_execute_request_example_biome, TEST_TERRAIN_DIR, TEST_TERRAIN_NAME,
    };
    use crate::common::test_utils::{TEST_SESSION_ID, TEST_TIMESTAMP_NUMERIC};
    use crate::common::types::command::Command;
    use crate::common::types::paths::DaemonPaths;
    use crate::common::types::terrain_state::test_utils::{
        terrain_state_after_activate, terrain_state_after_added_command,
        terrain_state_after_construct, terrain_state_after_construct_failed,
        terrain_state_after_deactivate_after_succeeded, terrain_state_execute_no_session,
    };
    use crate::common::types::terrain_state::{CommandStatus, TerrainState};
    use crate::common::utils::{create_file, write_to_file};
    use crate::daemon::handlers::execute::{spawn_commands, CommandInfo};
    use crate::daemon::types::config::DaemonConfig;
    use crate::daemon::types::context::DaemonContext;
    use crate::daemon::types::history::History;
    use crate::daemon::types::state::State;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use tempfile::tempdir;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn create_state_for_construct_without_session() {
        let state_paths = tempdir().unwrap();

        let request = expected_execute_request_example_biome(None, true);
        let context = DaemonContext::new(
            false,
            DaemonConfig::default(),
            Arc::new(MockExecutor::new()),
            Default::default(),
            DaemonPaths::new(state_paths.path().to_str().unwrap()),
        )
        .await;

        spawn_commands(request, Arc::new(context)).await.unwrap();

        let terrain_state_file = state_paths.path().join(format!(
            "{TEST_TERRAIN_NAME}/{TEST_TIMESTAMP_NUMERIC}/{TERRAIN_STATE_FILE_NAME}"
        ));
        assert!(terrain_state_file.exists());
        let actual_state: TerrainState =
            serde_json::from_reader(fs::File::open(&terrain_state_file).unwrap()).unwrap();
        assert_eq!(
            actual_state,
            terrain_state_execute_no_session(true, CommandStatus::Starting)
        );

        let history_file = state_paths
            .path()
            .join(format!("{TEST_TERRAIN_NAME}/{TERRAIN_HISTORY_FILE_NAME}"));
        assert!(history_file.exists());
        let history_contents = fs::read_to_string(&history_file).unwrap();
        assert_eq!(
            history_contents,
            format!("{TEST_TIMESTAMP_NUMERIC}\n\n\n\n")
        );
    }

    #[tokio::test]
    async fn add_construct_with_new_timestamp() {
        let state_paths = tempdir().unwrap();
        let new_timestamp = "1970-01-01_00:00:01".to_string();
        let is_auto_apply = true;
        let auto_apply = AutoApply::All;

        let mut request =
            expected_execute_request_example_biome(Some(TEST_SESSION_ID.to_string()), true);
        request.timestamp = new_timestamp.clone();

        let context = DaemonContext::new(
            false,
            DaemonConfig::default(),
            Arc::new(MockExecutor::new()),
            Default::default(),
            DaemonPaths::new(state_paths.path().to_str().unwrap()),
        )
        .await;

        // setup previous state with constructors already added
        let terrain_state_file = state_paths.path().join(format!(
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

        spawn_commands(request, Arc::new(context)).await.unwrap();

        let actual_state: TerrainState =
            serde_json::from_str(&fs::read_to_string(&terrain_state_file).unwrap()).unwrap();
        assert_eq!(
            actual_state,
            terrain_state_after_added_command(
                state_paths.path().to_str().unwrap(),
                TEST_SESSION_ID.to_string(),
                is_auto_apply,
                &auto_apply,
                true,
                new_timestamp
            )
        );

        let history_file = state_paths
            .path()
            .join(format!("{TEST_TERRAIN_NAME}/{TERRAIN_HISTORY_FILE_NAME}"));
        assert!(history_file.exists());
        let history_contents = fs::read_to_string(&history_file).unwrap();
        assert_eq!(history_contents, format!("{TEST_SESSION_ID}\n\n\n\n"));
    }

    #[tokio::test]
    async fn does_not_add_construct_with_same_timestamp() {
        let state_paths = tempdir().unwrap();
        let is_auto_apply = true;
        let auto_apply = AutoApply::All;

        let context = DaemonContext::new(
            false,
            DaemonConfig::default(),
            Arc::new(MockExecutor::new()),
            Default::default(),
            DaemonPaths::new(state_paths.path().to_str().unwrap()),
        )
        .await;

        let request =
            expected_execute_request_example_biome(Some(TEST_SESSION_ID.to_string()), true);

        // setup previous state with constructors already added
        let terrain_state_file = state_paths.path().join(format!(
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

        spawn_commands(request, Arc::new(context)).await.unwrap();

        let actual_state: TerrainState =
            serde_json::from_str(&fs::read_to_string(&terrain_state_file).unwrap()).unwrap();
        assert_eq!(
            actual_state,
            terrain_state_after_construct(TEST_SESSION_ID.to_string(), is_auto_apply, &auto_apply)
        );

        let history_file = state_paths
            .path()
            .join(format!("{TEST_TERRAIN_NAME}/{TERRAIN_HISTORY_FILE_NAME}"));
        assert!(history_file.exists());
        let history_contents = fs::read_to_string(&history_file).unwrap();
        assert_eq!(history_contents, format!("{TEST_SESSION_ID}\n\n\n\n"));
    }

    #[tokio::test]
    async fn create_state_for_destruct_without_session() {
        let state_paths = tempdir().unwrap();

        let request = expected_execute_request_example_biome(None, false);
        let context = DaemonContext::new(
            false,
            DaemonConfig::default(),
            Arc::new(MockExecutor::new()),
            Default::default(),
            DaemonPaths::new(state_paths.path().to_str().unwrap()),
        )
        .await;

        spawn_commands(request, Arc::new(context)).await.unwrap();

        let terrain_state_file = state_paths.path().join(format!(
            "{TEST_TERRAIN_NAME}/19700101000000/{TERRAIN_STATE_FILE_NAME}"
        ));
        assert!(terrain_state_file.exists());
        let actual_state: TerrainState =
            serde_json::from_reader(fs::File::open(&terrain_state_file).unwrap()).unwrap();
        assert_eq!(
            actual_state,
            terrain_state_execute_no_session(false, CommandStatus::Starting)
        );

        let history_file = state_paths
            .path()
            .join(format!("{TEST_TERRAIN_NAME}/{TERRAIN_HISTORY_FILE_NAME}"));
        assert!(history_file.exists());
        let history_contents = fs::read_to_string(&history_file).unwrap();
        assert_eq!(
            history_contents,
            format!("{TEST_TIMESTAMP_NUMERIC}\n\n\n\n")
        );
    }

    #[tokio::test]
    async fn add_destruct_with_new_timestamp() {
        let state_paths = tempdir().unwrap();
        let new_timestamp = "1970-01-01_00:00:01".to_string();
        let is_auto_apply = true;
        let auto_apply = AutoApply::All;

        let mut request =
            expected_execute_request_example_biome(Some(TEST_SESSION_ID.to_string()), false);
        request.timestamp = new_timestamp.clone();

        let context = DaemonContext::new(
            false,
            DaemonConfig::default(),
            Arc::new(MockExecutor::new()),
            Default::default(),
            DaemonPaths::new(state_paths.path().to_str().unwrap()),
        )
        .await;

        // setup previous state with constructors already added
        let terrain_state_file = state_paths.path().join(format!(
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

        spawn_commands(request, Arc::new(context)).await.unwrap();

        let actual_state: TerrainState =
            serde_json::from_str(&fs::read_to_string(&terrain_state_file).unwrap()).unwrap();
        assert_eq!(
            actual_state,
            terrain_state_after_added_command(
                state_paths.path().to_str().unwrap(),
                TEST_SESSION_ID.to_string(),
                is_auto_apply,
                &auto_apply,
                false,
                new_timestamp,
            )
        );

        let history_file = state_paths
            .path()
            .join(format!("{TEST_TERRAIN_NAME}/{TERRAIN_HISTORY_FILE_NAME}"));
        assert!(history_file.exists());
        let history_contents = fs::read_to_string(&history_file).unwrap();
        assert_eq!(history_contents, format!("{TEST_SESSION_ID}\n\n\n\n"));
    }

    #[tokio::test]
    async fn does_not_add_destruct_with_same_timestamp() {
        let state_paths = tempdir().unwrap();
        let is_auto_apply = true;
        let auto_apply = AutoApply::All;

        let context = DaemonContext::new(
            false,
            DaemonConfig::default(),
            Arc::new(MockExecutor::new()),
            Default::default(),
            DaemonPaths::new(state_paths.path().to_str().unwrap()),
        )
        .await;

        let request =
            expected_execute_request_example_biome(Some(TEST_SESSION_ID.to_string()), false);

        // setup previous state with constructors already added
        let terrain_state_file = state_paths.path().join(format!(
            "{TEST_TERRAIN_NAME}/{TEST_SESSION_ID}/{TERRAIN_STATE_FILE_NAME}"
        ));
        let old_state = terrain_state_after_deactivate_after_succeeded(
            state_paths.path().to_str().unwrap(),
            TEST_SESSION_ID.to_string(),
            is_auto_apply,
            &auto_apply,
        );

        let mut state_file = create_file(&terrain_state_file).await.unwrap();
        write_to_file(
            &mut state_file,
            serde_json::to_string_pretty(&old_state).unwrap(),
        )
        .await
        .unwrap();

        spawn_commands(request, Arc::new(context)).await.unwrap();

        let actual_state: TerrainState =
            serde_json::from_str(&fs::read_to_string(&terrain_state_file).unwrap()).unwrap();
        assert_eq!(
            actual_state,
            terrain_state_after_deactivate_after_succeeded(
                state_paths.path().to_str().unwrap(),
                TEST_SESSION_ID.to_string(),
                is_auto_apply,
                &auto_apply
            )
        );

        let history_file = state_paths
            .path()
            .join(format!("{TEST_TERRAIN_NAME}/{TERRAIN_HISTORY_FILE_NAME}"));
        assert!(history_file.exists());
        let history_contents = fs::read_to_string(&history_file).unwrap();
        assert_eq!(history_contents, format!("{TEST_SESSION_ID}\n\n\n\n"));
    }

    #[tokio::test]
    async fn throws_error_when_terrain_state_does_not_exist() {
        let state_paths = tempdir().unwrap();

        let request =
            expected_execute_request_example_biome(Some(TEST_SESSION_ID.to_string()), true);
        let context = DaemonContext::new(
            false,
            DaemonConfig::default(),
            Arc::new(MockExecutor::new()),
            Default::default(),
            DaemonPaths::new(state_paths.path().to_str().unwrap()),
        )
        .await;

        let error = spawn_commands(request, Arc::new(context))
            .await
            .unwrap_err()
            .to_string();

        assert_eq!(error, "failed to add commands to state manager");
    }

    #[tokio::test]
    async fn executes_command_and_updates_status_success() {
        let state_paths = tempdir().unwrap();
        let state_dir_path = state_paths.path().to_path_buf();

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

        let state = State::read(&state_path).await.unwrap();
        let history = History::read(state_dir_path.to_str().unwrap(), TEST_TERRAIN_NAME, 5)
            .await
            .unwrap();

        let (command, log_path) = old_state
            .get_constructors(TEST_TIMESTAMP)
            .unwrap()
            .remove(0)
            .command_and_log_path();

        let envs = Arc::new(expected_env_vars_example_biome(Path::new(TEST_TERRAIN_DIR)));

        let executor = AssertExecutor::with(MockExecutor::default())
            .async_spawn_with_log(
                ExpectedCommand {
                    command: Command::new(
                        "/bin/bash".to_string(),
                        vec![
                            "-c".to_string(),
                            "${PWD}/tests/scripts/print_num_for_10_sec".to_string(),
                        ],
                        Some(PathBuf::from(TEST_TERRAIN_DIR)),
                    ),
                    exit_code: 0,
                    should_error: false,
                    output: "".to_string(),
                },
                Some(envs.clone()),
                log_path.clone(),
                1,
            )
            .successfully();

        super::spawn_command(
            Arc::new(executor),
            Arc::new(RwLock::new(history)),
            Arc::new(RwLock::new(state)),
            CommandInfo {
                index: 0,
                command,
                envs,
                is_constructor: true,
                timestamp: TEST_TIMESTAMP.to_string(),
                log_path,
            },
        )
        .await
        .unwrap();

        let actual_state: TerrainState =
            serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
        assert_eq!(
            actual_state,
            terrain_state_after_construct(TEST_SESSION_ID.to_string(), is_auto_apply, &auto_apply)
        );

        let history_contents = fs::read_to_string(&history_path).unwrap();
        assert_eq!(history_contents, format!("{TEST_SESSION_ID}\n\n\n\n"));
    }

    #[tokio::test]
    async fn executes_command_and_updates_status_error() {
        let state_paths = tempdir().unwrap();
        let state_dir_path = state_paths.path().to_path_buf();

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

        let state = State::read(&state_path).await.unwrap();
        let history = History::read(state_dir_path.to_str().unwrap(), TEST_TERRAIN_NAME, 5)
            .await
            .unwrap();

        let (command, log_path) = old_state
            .get_constructors(TEST_TIMESTAMP)
            .unwrap()
            .remove(0)
            .command_and_log_path();

        let envs = Arc::new(expected_env_vars_example_biome(Path::new(TEST_TERRAIN_DIR)));

        let executor = AssertExecutor::with(MockExecutor::default())
            .async_spawn_with_log(
                ExpectedCommand {
                    command: Command::new(
                        "/bin/bash".to_string(),
                        vec![
                            "-c".to_string(),
                            "${PWD}/tests/scripts/print_num_for_10_sec".to_string(),
                        ],
                        Some(PathBuf::from(TEST_TERRAIN_DIR)),
                    ),
                    exit_code: 1,
                    should_error: false,
                    output: "".to_string(),
                },
                Some(envs.clone()),
                log_path.clone(),
                1,
            )
            .successfully();

        let error = super::spawn_command(
            Arc::new(executor),
            Arc::new(RwLock::new(history)),
            Arc::new(RwLock::new(state)),
            CommandInfo {
                index: 0,
                command,
                envs,
                is_constructor: true,
                timestamp: TEST_TIMESTAMP.to_string(),
                log_path,
            },
        )
        .await
        .unwrap_err()
        .to_string();

        assert_eq!(
            error,
            r#"command: `/bin/bash -c ${PWD}/tests/scripts/print_num_for_10_sec` in '/tmp/terrain_dir' exited with code Some(1)"#
        );

        let actual_state: TerrainState =
            serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
        assert_eq!(
            actual_state,
            terrain_state_after_construct_failed(
                TEST_SESSION_ID.to_string(),
                is_auto_apply,
                &auto_apply
            )
        );

        let history_contents = fs::read_to_string(&history_path).unwrap();
        assert_eq!(history_contents, format!("{TEST_SESSION_ID}\n\n\n\n"));
    }
}
