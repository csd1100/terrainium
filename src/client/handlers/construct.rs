use crate::client::args::BiomeArg;
use crate::client::handlers::background;
#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use crate::common::constants::CONSTRUCTORS;
use anyhow::Result;

pub async fn handle(
    context: Context,
    biome_arg: Option<BiomeArg>,
    terrain: Terrain,
    client: Option<Client>,
) -> Result<()> {
    background::handle(&context, CONSTRUCTORS, terrain, biome_arg, None, client).await
}

#[cfg(test)]
mod tests {
    use crate::client::shell::Zsh;
    use crate::client::types::context::Context;
    use crate::client::types::terrain::Terrain;
    use crate::client::utils::{AssertExecuteRequest, RunCommand};
    use crate::common::constants::{CONSTRUCTORS, TERRAIN_DIR, TERRAIN_SELECTED_BIOME};
    use crate::common::execute::MockCommandToRun;
    use crate::common::types::pb;
    use crate::common::types::pb::ExecuteResponse;
    use prost_types::Any;
    use std::fs::copy;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[tokio::test]
    async fn construct_send_message_to_daemon() {
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
            .operation(CONSTRUCTORS)
            .is_activated_as(false)
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
                    .with_env(TERRAIN_SELECTED_BIOME, "example_biome"),
            )
            .sent();

        super::handle(context, None, Terrain::example(), Some(expected_request))
            .await
            .expect("no error to be thrown");
    }

    #[tokio::test]
    async fn construct_send_message_to_daemon_and_error() {
        let current_dir = tempdir().expect("failed to create tempdir");

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");

        copy("./tests/data/terrain.example.toml", &terrain_toml)
            .expect("copy to terrain to test dir");

        let expected_request = AssertExecuteRequest::with()
            .operation(CONSTRUCTORS)
            .is_activated_as(false)
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
                    .with_env(TERRAIN_SELECTED_BIOME, "example_biome"),
            )
            .sent();

        let context = Context::build(
            current_dir.path().into(),
            PathBuf::new(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(MockCommandToRun::default()),
        );

        let err = super::handle(context, None, Terrain::example(), Some(expected_request))
            .await
            .expect_err("to be thrown");

        assert_eq!(
            err.to_string(),
            "error response from daemon failed to execute"
        );
    }

    #[tokio::test]
    async fn construct_does_not_send_message_to_daemon_no_background_process_defined() {
        let context = Context::build(
            PathBuf::new(),
            PathBuf::new(),
            PathBuf::new(),
            Zsh::build(MockCommandToRun::default()),
        );

        let expected_request = AssertExecuteRequest::not_sent();
        super::handle(context, None, Terrain::default(), Some(expected_request))
            .await
            .expect("no error to be thrown");
    }
}
