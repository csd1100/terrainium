use crate::client::args::BiomeArg;
use crate::client::handlers::background::execute_request;
#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::environment::Environment;
use crate::client::types::proto::ProtoRequest;
use crate::client::types::terrain::{AutoApply, Terrain};
use crate::common::constants::{
    get_terrainiumd_socket, TERRAIN_AUTO_APPLY, TERRAIN_SELECTED_BIOME,
};
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
        Client::new(PathBuf::from(get_terrainiumd_socket())).await?
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
            auto_apply.is_background_enabled()
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
    use crate::client::test_utils::{restore_env_var, set_env_var};
    use crate::client::types::context::Context;
    use crate::client::types::proto::ProtoRequest;
    use crate::client::types::terrain::{AutoApply, Terrain};
    use crate::common::constants::{
        EXAMPLE_BIOME, NONE, TERRAIN_AUTO_APPLY, TERRAIN_SELECTED_BIOME, TEST_TIMESTAMP,
    };
    use crate::common::execute::MockExecutor;
    use crate::common::test_utils;
    use crate::common::test_utils::{TEST_SESSION_ID, TEST_TERRAIN_DIR, TEST_TERRAIN_NAME};
    use crate::common::types::pb;
    use serial_test::serial;
    use std::env::VarError;
    use std::path::Path;

    const TERRAIN_NOT_ACTIVE_ERR: &str =
        "no active terrain found, use 'terrainium enter' command to activate a terrain.";

    fn expected_request_none() -> pb::Deactivate {
        pb::Deactivate {
            session_id: TEST_SESSION_ID.to_string(),
            terrain_name: TEST_TERRAIN_NAME.to_string(),
            end_timestamp: TEST_TIMESTAMP.to_string(),
            destructors: None,
        }
    }

    #[tokio::test]
    async fn should_throw_an_error_if_terrain_session_id_not_set() {
        // by default session id is not set for context created using build
        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::default());

        let actual_error = super::handle(context, Terrain::example(), None)
            .await
            .unwrap_err()
            .to_string();

        assert_eq!(actual_error, TERRAIN_NOT_ACTIVE_ERR);
    }

    #[serial]
    #[tokio::test]
    async fn should_throw_an_error_if_terrain_selected_biome_not_set() {
        let selected_biome: std::result::Result<String, VarError>;
        unsafe {
            selected_biome = set_env_var(TERRAIN_SELECTED_BIOME, None);
        }

        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::default())
            .set_session_id(TEST_SESSION_ID);

        let actual_error = super::handle(context, Terrain::example(), None)
            .await
            .unwrap_err()
            .to_string();

        assert_eq!(actual_error, TERRAIN_NOT_ACTIVE_ERR);

        unsafe {
            restore_env_var(TERRAIN_SELECTED_BIOME, selected_biome);
        }
    }

    #[serial]
    #[tokio::test]
    async fn send_request_for_example_biome() {
        let selected_biome: std::result::Result<String, VarError>;
        unsafe {
            selected_biome = set_env_var(TERRAIN_SELECTED_BIOME, Some(EXAMPLE_BIOME));
        }

        let client = ExpectClient::send(ProtoRequest::Deactivate(
            test_utils::expected_deactivate_request_example_biome(TEST_SESSION_ID),
        ))
        .successfully();

        let context = Context::build(
            Path::new(TEST_TERRAIN_DIR),
            Path::new(""),
            false,
            MockExecutor::default(),
        )
        .set_session_id(TEST_SESSION_ID);

        super::handle(context, Terrain::example(), Some(client))
            .await
            .unwrap();

        unsafe {
            restore_env_var(TERRAIN_SELECTED_BIOME, selected_biome);
        }
    }

    #[serial]
    #[tokio::test]
    async fn send_request_for_none() {
        let selected_biome: std::result::Result<String, VarError>;
        unsafe {
            selected_biome = set_env_var(TERRAIN_SELECTED_BIOME, Some(NONE));
        }

        let client =
            ExpectClient::send(ProtoRequest::Deactivate(expected_request_none())).successfully();

        let context = Context::build(
            Path::new(TEST_TERRAIN_DIR),
            Path::new(""),
            false,
            MockExecutor::default(),
        )
        .set_session_id(TEST_SESSION_ID);

        super::handle(context, Terrain::example(), Some(client))
            .await
            .unwrap();

        unsafe {
            restore_env_var(TERRAIN_SELECTED_BIOME, selected_biome);
        }
    }

    #[serial]
    #[tokio::test]
    async fn send_request_for_auto_apply_enabled_but_not_background() {
        let selected_biome: std::result::Result<String, VarError>;
        let auto_apply: std::result::Result<String, VarError>;

        unsafe {
            selected_biome = set_env_var(TERRAIN_SELECTED_BIOME, Some(EXAMPLE_BIOME));
            auto_apply = set_env_var(TERRAIN_AUTO_APPLY, Some(&AutoApply::Enabled.to_string()));
        }

        let client =
            ExpectClient::send(ProtoRequest::Deactivate(expected_request_none())).successfully();

        let context = Context::build(
            Path::new(TEST_TERRAIN_DIR),
            Path::new(""),
            false,
            MockExecutor::default(),
        )
        .set_session_id(TEST_SESSION_ID);

        super::handle(context, Terrain::example(), Some(client))
            .await
            .unwrap();

        unsafe {
            restore_env_var(TERRAIN_SELECTED_BIOME, selected_biome);
            restore_env_var(TERRAIN_AUTO_APPLY, auto_apply);
        }
    }

    #[serial]
    #[tokio::test]
    async fn send_request_for_auto_apply_replace_but_not_background() {
        let selected_biome: std::result::Result<String, VarError>;
        let auto_apply: std::result::Result<String, VarError>;

        unsafe {
            selected_biome = set_env_var(TERRAIN_SELECTED_BIOME, Some(EXAMPLE_BIOME));
            auto_apply = set_env_var(TERRAIN_AUTO_APPLY, Some(&AutoApply::Replace.to_string()));
        }

        let client =
            ExpectClient::send(ProtoRequest::Deactivate(expected_request_none())).successfully();

        let context = Context::build(
            Path::new(TEST_TERRAIN_DIR),
            Path::new(""),
            false,
            MockExecutor::default(),
        )
        .set_session_id(TEST_SESSION_ID);

        super::handle(context, Terrain::example(), Some(client))
            .await
            .unwrap();

        unsafe {
            restore_env_var(TERRAIN_SELECTED_BIOME, selected_biome);
            restore_env_var(TERRAIN_AUTO_APPLY, auto_apply);
        }
    }

    #[serial]
    #[tokio::test]
    async fn should_throw_an_error_if_error_response() {
        let selected_biome: std::result::Result<String, VarError>;
        unsafe {
            selected_biome = set_env_var(TERRAIN_SELECTED_BIOME, Some(NONE));
        }

        let context = Context::build(Path::new(""), Path::new(""), false, MockExecutor::default())
            .set_session_id(TEST_SESSION_ID);

        let client = ExpectClient::send(ProtoRequest::Deactivate(expected_request_none()))
            .with_returning_error("failed to parse the request");

        let actual_error = super::handle(context, Terrain::example(), Some(client))
            .await
            .unwrap_err()
            .to_string();

        assert_eq!(actual_error, "failed to parse the request");

        unsafe {
            restore_env_var(TERRAIN_SELECTED_BIOME, selected_biome);
        }
    }
}
