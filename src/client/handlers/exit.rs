use crate::client::args::BiomeArg;
use crate::client::handlers::background;
#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use crate::common::constants::{DESTRUCTORS, TERRAIN_AUTO_APPLY, TERRAIN_SELECTED_BIOME};
use anyhow::{anyhow, Context as AnyhowContext, Result};
use std::collections::BTreeMap;
use std::env;

pub async fn handle(context: Context, terrain: Terrain, client: Option<Client>) -> Result<()> {
    let session_id = context.session_id();
    let selected_biome = env::var(TERRAIN_SELECTED_BIOME).unwrap_or_default();

    if session_id.is_empty() || selected_biome.is_empty() {
        return Err(anyhow!(
            "no active terrain found, use 'terrainium enter' command to activate a terrain."
        ));
    }

    if should_run_destructor() {
        background::handle(
            &context,
            DESTRUCTORS,
            terrain,
            Some(BiomeArg::Some(selected_biome)),
            Some(BTreeMap::<String, String>::new()),
            client,
        )
        .await
        .context("failed to run destructors")?;
    }

    Ok(())
}

/// 'terrainium exit' should run background destructor commands only in following case:
/// 1. Auto-apply is disabled
/// 2. Auto-apply is enabled but background flag is also turned on
fn should_run_destructor() -> bool {
    let auto_apply = env::var(TERRAIN_AUTO_APPLY);
    match auto_apply {
        Ok(auto_apply) => auto_apply == "all" || auto_apply == "background",
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use crate::client::shell::Zsh;
    use crate::client::types::context::Context;
    use crate::client::types::terrain::tests::set_auto_apply;
    use crate::client::types::terrain::Terrain;
    use crate::client::utils::{restore_env_var, set_env_var, AssertExecuteRequest, RunCommand};
    use crate::common::constants::{
        DESTRUCTORS, TERRAIN_AUTO_APPLY, TERRAIN_DIR, TERRAIN_SELECTED_BIOME, TERRAIN_SESSION_ID,
    };
    use crate::common::execute::MockCommandToRun;
    use crate::common::types::pb;
    use crate::common::types::pb::ExecuteResponse;
    use prost_types::Any;
    use serial_test::serial;
    use std::fs::copy;
    use std::path::PathBuf;
    use tempfile::tempdir;
    //
    // RUN THESE TESTS IN SERIAL BECAUSE ENV VARS IN PARALLEL TESTS GET MESSED UP
    //

    #[serial]
    #[tokio::test]
    async fn exit_send_message_to_daemon() {
        let orig_selected_biome = set_env_var(
            TERRAIN_SELECTED_BIOME.to_string(),
            Some("example_biome".to_string()),
        );
        let orig_auto_apply = set_env_var(TERRAIN_AUTO_APPLY.to_string(), None);

        let current_dir = tempdir().expect("failed to create tempdir");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("copy to terrain to test dir");

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(MockCommandToRun::default()),
        );

        let expected_request = AssertExecuteRequest::with()
            .is_activated_as(true)
            .operation(DESTRUCTORS)
            .with_expected_reply(
                Any::from_msg(&ExecuteResponse {}).expect("to be converted to any"),
            )
            .terrain_name("terrainium")
            .biome_name("example_biome")
            .toml_path(terrain_toml.to_str().unwrap())
            .with_command(
                RunCommand::with_exe("/bin/bash")
                    .with_arg("-c")
                    .with_arg("$PWD/tests/scripts/print_num_for_10_sec")
                    .with_cwd(current_dir.path())
                    .with_env("EDITOR", "nvim")
                    .with_env("NULL_POINTER", "${NULL}")
                    .with_env("PAGER", "less")
                    .with_env("ENV_VAR", "overridden_env_val")
                    .with_env(
                        "NESTED_POINTER",
                        "overridden_env_val-overridden_env_val-${NULL}",
                    )
                    .with_env("POINTER_ENV_VAR", "overridden_env_val")
                    .with_env(TERRAIN_DIR, current_dir.path().to_str().unwrap())
                    .with_env(TERRAIN_SESSION_ID, "some")
                    .with_env(TERRAIN_SELECTED_BIOME, "example_biome"),
            )
            .sent();

        super::handle(context, Terrain::example(), Some(expected_request))
            .await
            .expect("no error to be thrown");

        restore_env_var(TERRAIN_SELECTED_BIOME.to_string(), orig_selected_biome);
        restore_env_var(TERRAIN_AUTO_APPLY.to_string(), orig_auto_apply);
    }

    #[serial]
    #[tokio::test]
    async fn exit_send_message_to_daemon_and_error() {
        let orig_selected_biome = set_env_var(
            TERRAIN_SELECTED_BIOME.to_string(),
            Some("example_biome".to_string()),
        );
        let orig_auto_apply = set_env_var(TERRAIN_AUTO_APPLY.to_string(), None);

        let current_dir = tempdir().expect("failed to create tempdir");
        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("copy to terrain to test dir");

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(MockCommandToRun::default()),
        );

        let expected_request = AssertExecuteRequest::with()
            .is_activated_as(true)
            .operation(DESTRUCTORS)
            .with_expected_reply(
                Any::from_msg(&pb::Error {
                    error_message: "failed to execute".to_string(),
                })
                .expect("to be converted to any"),
            )
            .terrain_name("terrainium")
            .biome_name("example_biome")
            .toml_path(terrain_toml.to_str().unwrap())
            .with_command(
                RunCommand::with_exe("/bin/bash")
                    .with_arg("-c")
                    .with_arg("$PWD/tests/scripts/print_num_for_10_sec")
                    .with_cwd(current_dir.path())
                    .with_env("EDITOR", "nvim")
                    .with_env("NULL_POINTER", "${NULL}")
                    .with_env("PAGER", "less")
                    .with_env("ENV_VAR", "overridden_env_val")
                    .with_env(
                        "NESTED_POINTER",
                        "overridden_env_val-overridden_env_val-${NULL}",
                    )
                    .with_env("POINTER_ENV_VAR", "overridden_env_val")
                    .with_env(TERRAIN_DIR, current_dir.path().to_str().unwrap())
                    .with_env(TERRAIN_SESSION_ID, "some")
                    .with_env(TERRAIN_SELECTED_BIOME, "example_biome"),
            )
            .sent();

        let err = super::handle(context, Terrain::example(), Some(expected_request))
            .await
            .expect_err("to be thrown");

        assert_eq!(err.to_string(), "failed to run destructors");

        restore_env_var(TERRAIN_SELECTED_BIOME.to_string(), orig_selected_biome);
        restore_env_var(TERRAIN_AUTO_APPLY.to_string(), orig_auto_apply);
    }

    #[serial]
    #[tokio::test]
    async fn exit_does_not_send_message_to_daemon_auto_apply_without_background() {
        let orig_selected_biome = set_env_var(
            TERRAIN_SELECTED_BIOME.to_string(),
            Some("example_biome".to_string()),
        );
        let orig_auto_apply =
            set_env_var(TERRAIN_AUTO_APPLY.to_string(), Some("enabled".to_string()));

        let current_dir = tempdir().expect("failed to create tempdir");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy(
            "./tests/data/terrain.example.auto_apply.enabled.toml",
            &terrain_toml,
        )
        .expect("copy to terrain to test dir");

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(MockCommandToRun::default()),
        );

        let expected_request = AssertExecuteRequest::not_sent();

        let mut terrain = Terrain::example();
        set_auto_apply(&mut terrain, "enable");

        super::handle(context, terrain, Some(expected_request))
            .await
            .expect("no error to be thrown");

        restore_env_var(TERRAIN_SELECTED_BIOME.to_string(), orig_selected_biome);
        restore_env_var(TERRAIN_AUTO_APPLY.to_string(), orig_auto_apply);
    }

    #[serial]
    #[tokio::test]
    async fn exit_does_send_message_to_daemon_auto_apply() {
        let orig_selected_biome = set_env_var(
            TERRAIN_SELECTED_BIOME.to_string(),
            Some("example_biome".to_string()),
        );
        let orig_auto_apply = set_env_var(TERRAIN_AUTO_APPLY.to_string(), Some("all".to_string()));

        let current_dir = tempdir().expect("failed to create tempdir");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("copy to terrain to test dir");

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(MockCommandToRun::default()),
        );

        let expected_request = AssertExecuteRequest::with()
            .is_activated_as(true)
            .operation(DESTRUCTORS)
            .with_expected_reply(
                Any::from_msg(&ExecuteResponse {}).expect("to be converted to any"),
            )
            .terrain_name("terrainium")
            .biome_name("example_biome")
            .toml_path(terrain_toml.to_str().unwrap())
            .with_command(
                RunCommand::with_exe("/bin/bash")
                    .with_arg("-c")
                    .with_arg("$PWD/tests/scripts/print_num_for_10_sec")
                    .with_cwd(current_dir.path())
                    .with_env("EDITOR", "nvim")
                    .with_env("NULL_POINTER", "${NULL}")
                    .with_env("PAGER", "less")
                    .with_env("ENV_VAR", "overridden_env_val")
                    .with_env(
                        "NESTED_POINTER",
                        "overridden_env_val-overridden_env_val-${NULL}",
                    )
                    .with_env("POINTER_ENV_VAR", "overridden_env_val")
                    .with_env(TERRAIN_DIR, current_dir.path().to_str().unwrap())
                    .with_env(TERRAIN_SESSION_ID, "some")
                    .with_env(TERRAIN_SELECTED_BIOME, "example_biome"),
            )
            .sent();

        let mut terrain = Terrain::example();
        set_auto_apply(&mut terrain, "all");

        super::handle(context, terrain, Some(expected_request))
            .await
            .expect("no error to be thrown");

        restore_env_var(TERRAIN_SELECTED_BIOME.to_string(), orig_selected_biome);
        restore_env_var(TERRAIN_AUTO_APPLY.to_string(), orig_auto_apply);
    }

    #[serial]
    #[tokio::test]
    async fn exit_does_not_send_message_to_daemon_auto_apply_with_empty_background_commands() {
        let orig_selected_biome = set_env_var(
            TERRAIN_SELECTED_BIOME.to_string(),
            Some("example_biome".to_string()),
        );
        let orig_auto_apply = set_env_var(
            TERRAIN_AUTO_APPLY.to_string(),
            Some("background".to_string()),
        );

        let current_dir = tempdir().expect("failed to create tempdir");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy(
            "./tests/data/terrain.example.without.background.auto_apply.background.toml",
            &terrain_toml,
        )
        .expect("copy to terrain to test dir");

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(MockCommandToRun::default()),
        );

        let expected_request = AssertExecuteRequest::not_sent();

        let terrain = Terrain::get_validated_and_fixed_terrain(&context).unwrap();

        super::handle(context, terrain, Some(expected_request))
            .await
            .expect("no error to be thrown");

        restore_env_var(TERRAIN_SELECTED_BIOME.to_string(), orig_selected_biome);
        restore_env_var(TERRAIN_AUTO_APPLY.to_string(), orig_auto_apply);
    }
}
