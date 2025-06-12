use crate::client::args::BiomeArg;
use crate::client::handlers::background::execute_request;
#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::environment::Environment;
use crate::client::types::proto::ProtoRequest;
use crate::client::types::terrain::{AutoApply, Terrain};
use crate::common::constants::{TERRAINIUMD_SOCKET, TERRAIN_AUTO_APPLY, TERRAIN_SELECTED_BIOME};
use crate::common::types::pb;
use crate::common::utils::timestamp;
use anyhow::{bail, Context as AnyhowContext, Result};
use std::env;
use std::path::PathBuf;
use std::str::FromStr;

pub async fn handle(context: Context, terrain: Terrain, client: Option<Client>) -> Result<()> {
    let session_id = context.session_id();
    let selected_biome = env::var(TERRAIN_SELECTED_BIOME).ok();

    if session_id.is_none() || selected_biome.is_none() {
        bail!("no active terrain found, use 'terrainium enter' command to activate a terrain.");
    }

    let mut client = if let Some(client) = client {
        client
    } else {
        Client::new(PathBuf::from(TERRAINIUMD_SOCKET)).await?
    };

    client
        .request(ProtoRequest::Deactivate(deactivate(
            terrain.name().to_string(),
            session_id.expect("session id to be present").to_string(),
            selected_biome.unwrap(),
            terrain,
            context,
        )?))
        .await?;
    Ok(())
}

/// 'terrainium exit' should run background destructor commands only in following case:
/// 1. Auto-apply is disabled i.e. TERRAIN_AUTO_APPLY env var is not set i.e.
///    user activated terrain manually
/// 2. Auto-apply is enabled and background flag is also turned on
fn should_run_destructor() -> bool {
    let auto_apply = env::var(TERRAIN_AUTO_APPLY);
    match auto_apply {
        Ok(auto_apply) => {
            let auto_apply = AutoApply::from_str(&auto_apply)
                .expect("expect auto-apply to be converted from string");
            auto_apply.is_background() || auto_apply.is_all()
        }
        Err(_) => true,
    }
}

fn deactivate(
    terrain_name: String,
    session_id: String,
    selected_biome: String,
    terrain: Terrain,
    context: Context,
) -> Result<pb::Deactivate> {
    let end_timestamp = timestamp();
    let destructors = if should_run_destructor() {
        let environment = Environment::from(
            &terrain,
            BiomeArg::from_str(&selected_biome).unwrap(),
            context.terrain_dir(),
        )
        .context("failed to generate environment")?;
        execute_request(&context, environment, false, end_timestamp.clone())?
    } else {
        None
    };

    Ok(pb::Deactivate {
        session_id,
        terrain_name,
        end_timestamp,
        destructors,
    })
}

#[cfg(test)]
mod tests {
    use crate::client::test_utils::assertions::client::ExpectClient;
    use crate::client::test_utils::constants::{
        TEST_SESSION_ID, TEST_TERRAIN_NAME, TEST_TIMESTAMP,
    };
    use crate::client::test_utils::{
        expected_execute_request_example_biome, restore_env_var, set_env_var,
    };
    use crate::client::types::config::Config;
    use crate::client::types::context::Context;
    use crate::client::types::proto::ProtoRequest;
    use crate::client::types::terrain::{AutoApply, Terrain};
    use crate::common::constants::{
        EXAMPLE_BIOME, NONE, TERRAIN_AUTO_APPLY, TERRAIN_SELECTED_BIOME,
    };
    use crate::common::execute::MockExecutor;
    use crate::common::types::pb;
    use serial_test::serial;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    const TERRAIN_NOT_ACTIVE_ERR: &str =
        "no active terrain found, use 'terrainium enter' command to activate a terrain.";

    fn expected_request_example_biome(session_id: String, terrain_dir: &Path) -> pb::Deactivate {
        pb::Deactivate {
            session_id: session_id.clone(),
            terrain_name: TEST_TERRAIN_NAME.to_string(),
            end_timestamp: TEST_TIMESTAMP.to_string(),
            destructors: Some(expected_execute_request_example_biome(
                Some(session_id),
                terrain_dir,
                false,
            )),
        }
    }

    fn expected_request_none(session_id: String) -> pb::Deactivate {
        pb::Deactivate {
            session_id: session_id.clone(),
            terrain_name: TEST_TERRAIN_NAME.to_string(),
            end_timestamp: TEST_TIMESTAMP.to_string(),
            destructors: None,
        }
    }

    #[tokio::test]
    async fn should_throw_an_error_if_terrain_session_id_not_set() {
        // by default session id is not set for context created using build
        let context = Context::build(
            PathBuf::new(),
            PathBuf::new(),
            PathBuf::new(),
            Config::default(),
            MockExecutor::default(),
        );

        let actual_error = super::handle(context, Terrain::example(), None)
            .await
            .unwrap_err()
            .to_string();

        assert_eq!(actual_error, TERRAIN_NOT_ACTIVE_ERR);
    }

    #[serial]
    #[tokio::test]
    async fn should_throw_an_error_if_terrain_selected_biome_not_set() {
        let selected_biome = set_env_var(TERRAIN_SELECTED_BIOME.to_string(), None);

        let context = Context::build(
            PathBuf::new(),
            PathBuf::new(),
            PathBuf::new(),
            Config::default(),
            MockExecutor::default(),
        )
        .set_session_id(TEST_SESSION_ID.to_string());

        let actual_error = super::handle(context, Terrain::example(), None)
            .await
            .unwrap_err()
            .to_string();

        assert_eq!(actual_error, TERRAIN_NOT_ACTIVE_ERR);

        restore_env_var(TERRAIN_SELECTED_BIOME.to_string(), selected_biome);
    }

    #[serial]
    #[tokio::test]
    async fn send_request_for_example_biome() {
        let session_id = TEST_SESSION_ID.to_string();
        let selected_biome = set_env_var(
            TERRAIN_SELECTED_BIOME.to_string(),
            Some(EXAMPLE_BIOME.to_string()),
        );

        let terrain_dir = tempdir().unwrap();

        let client = ExpectClient::to_send(ProtoRequest::Deactivate(
            expected_request_example_biome(session_id.clone(), terrain_dir.path()),
        ))
        .successfully();

        let context = Context::build(
            terrain_dir.path().to_path_buf(),
            PathBuf::new(),
            terrain_dir.path().join("terrain.toml"),
            Config::default(),
            MockExecutor::default(),
        )
        .set_session_id(session_id);

        super::handle(context, Terrain::example(), Some(client))
            .await
            .unwrap();

        restore_env_var(TERRAIN_SELECTED_BIOME.to_string(), selected_biome);
    }

    #[serial]
    #[tokio::test]
    async fn send_request_for_none() {
        let session_id = "session_id".to_string();
        let selected_biome =
            set_env_var(TERRAIN_SELECTED_BIOME.to_string(), Some(NONE.to_string()));

        let terrain_dir = tempdir().unwrap();

        let client = ExpectClient::to_send(ProtoRequest::Deactivate(expected_request_none(
            session_id.clone(),
        )))
        .successfully();

        let context = Context::build(
            terrain_dir.path().to_path_buf(),
            PathBuf::new(),
            terrain_dir.path().join("terrain.toml"),
            Config::default(),
            MockExecutor::default(),
        )
        .set_session_id(session_id);

        super::handle(context, Terrain::example(), Some(client))
            .await
            .unwrap();

        restore_env_var(TERRAIN_SELECTED_BIOME.to_string(), selected_biome);
    }

    #[serial]
    #[tokio::test]
    async fn send_request_for_auto_apply_enabled_but_not_background() {
        let session_id = "session_id".to_string();
        let selected_biome = set_env_var(
            TERRAIN_SELECTED_BIOME.to_string(),
            Some(EXAMPLE_BIOME.to_string()),
        );
        let auto_apply = set_env_var(
            TERRAIN_AUTO_APPLY.to_string(),
            Some((&AutoApply::enabled()).into()),
        );

        let terrain_dir = tempdir().unwrap();

        let client = ExpectClient::to_send(ProtoRequest::Deactivate(expected_request_none(
            session_id.clone(),
        )))
        .successfully();

        let context = Context::build(
            terrain_dir.path().to_path_buf(),
            PathBuf::new(),
            terrain_dir.path().join("terrain.toml"),
            Config::default(),
            MockExecutor::default(),
        )
        .set_session_id(session_id);

        super::handle(context, Terrain::example(), Some(client))
            .await
            .unwrap();

        restore_env_var(TERRAIN_SELECTED_BIOME.to_string(), selected_biome);
        restore_env_var(TERRAIN_AUTO_APPLY.to_string(), auto_apply);
    }
    #[serial]
    #[tokio::test]
    async fn send_request_for_auto_apply_replace_but_not_background() {
        let session_id = "session_id".to_string();
        let selected_biome = set_env_var(
            TERRAIN_SELECTED_BIOME.to_string(),
            Some(EXAMPLE_BIOME.to_string()),
        );
        let auto_apply = set_env_var(
            TERRAIN_AUTO_APPLY.to_string(),
            Some((&AutoApply::replace()).into()),
        );

        let terrain_dir = tempdir().unwrap();

        let client = ExpectClient::to_send(ProtoRequest::Deactivate(expected_request_none(
            session_id.clone(),
        )))
        .successfully();

        let context = Context::build(
            terrain_dir.path().to_path_buf(),
            PathBuf::new(),
            terrain_dir.path().join("terrain.toml"),
            Config::default(),
            MockExecutor::default(),
        )
        .set_session_id(session_id);

        super::handle(context, Terrain::example(), Some(client))
            .await
            .unwrap();

        restore_env_var(TERRAIN_SELECTED_BIOME.to_string(), selected_biome);
        restore_env_var(TERRAIN_AUTO_APPLY.to_string(), auto_apply);
    }

    #[serial]
    #[tokio::test]
    async fn should_throw_an_error_if_error_response() {
        let selected_biome =
            set_env_var(TERRAIN_SELECTED_BIOME.to_string(), Some(NONE.to_string()));
        let session_id = TEST_SESSION_ID.to_string();

        let context = Context::build(
            PathBuf::new(),
            PathBuf::new(),
            PathBuf::new(),
            Config::default(),
            MockExecutor::default(),
        )
        .set_session_id(session_id.clone());

        let client =
            ExpectClient::to_send(ProtoRequest::Deactivate(expected_request_none(session_id)))
                .with_returning_error("failed to parse the request".to_string());

        let actual_error = super::handle(context, Terrain::example(), Some(client))
            .await
            .unwrap_err()
            .to_string();

        assert_eq!(actual_error, "failed to parse the request");

        restore_env_var(TERRAIN_SELECTED_BIOME.to_string(), selected_biome);
    }
}
