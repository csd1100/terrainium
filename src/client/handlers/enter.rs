use crate::client::args::{option_string_from, BiomeArg};
use crate::client::handlers::background;
use crate::client::shell::Shell;
#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::environment::Environment;
use crate::client::types::terrain::Terrain;
use crate::common::constants::{CONSTRUCTORS, TERRAIN_AUTO_APPLY, TERRAIN_ENABLED};
use anyhow::{Context as AnyhowContext, Result};

pub async fn handle(
    context: Context,
    biome_arg: Option<BiomeArg>,
    terrain: Terrain,
    auto_apply: bool,
    client: Option<Client>,
) -> Result<()> {
    let biome = option_string_from(&biome_arg);
    let (selected_name, _) = terrain.select_biome(&biome)?;
    let environment = Environment::from(&terrain, biome, context.terrain_dir())
        .context("failed to generate environment")?;

    let mut envs = environment.envs();
    envs.insert(TERRAIN_ENABLED.to_string(), "true".to_string());
    envs.append(&mut context.terrainium_envs().clone());

    let mut zsh_envs = context
        .shell()
        .generate_envs(&context, selected_name.to_string())?;
    envs.append(&mut zsh_envs);

    if auto_apply {
        envs.insert(
            TERRAIN_AUTO_APPLY.to_string(),
            terrain.auto_apply().clone().into(),
        );
    }

    if auto_apply && !terrain.auto_apply().is_background() {
        context
            .shell()
            .spawn(envs)
            .await
            .context("failed to enter terrain environment")?;
    } else {
        let result = tokio::join!(
            context.shell().spawn(envs.clone()),
            background::handle(
                &context,
                CONSTRUCTORS,
                terrain,
                biome_arg,
                Some(envs),
                client
            ),
        );

        if let Err(e) = result.0 {
            anyhow::bail!(
                "failed to spawn background processes while entering terrain environment: {e}",
            );
        }

        if let Err(e) = result.1 {
            anyhow::bail!("failed to spawn shell while entering terrain environment: {e}");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::client::shell::Zsh;
    use crate::client::types::context::Context;
    use crate::client::types::terrain::tests::set_auto_apply;
    use crate::client::types::terrain::Terrain;
    use crate::client::utils::{AssertExecuteRequest, ExpectShell, RunCommand};
    use crate::common::constants::{
        CONSTRUCTORS, FPATH, TERRAIN_AUTO_APPLY, TERRAIN_DIR, TERRAIN_ENABLED, TERRAIN_INIT_FN,
        TERRAIN_INIT_SCRIPT, TERRAIN_SELECTED_BIOME, TERRAIN_SESSION_ID,
    };
    use crate::common::types::pb::ExecuteResponse;
    use prost_types::Any;
    use tempfile::tempdir;
    use tokio::fs::copy;

    #[tokio::test]
    async fn enter_default() {
        let current_dir = tempdir().expect("Couldn't create temp dir");
        let central_dir = tempdir().expect("Couldn't create temp dir");

        let compiled_script = central_dir
            .path()
            .join("scripts")
            .join("terrain-example_biome.zwc");
        const EXISTING_FPATH: &str = "/some/path:/some/path2";
        let fpath = format!("{}:{EXISTING_FPATH}", compiled_script.display());

        let expected_shell_operations = ExpectShell::to()
            .execute(
                RunCommand::with_exe("/bin/zsh")
                    .with_arg("-c")
                    .with_arg("/bin/echo -n $FPATH")
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
                    .with_env(TERRAIN_INIT_FN, "terrain-example_biome.zsh")
                    .with_env(TERRAIN_ENABLED, "true")
                    .with_env(TERRAIN_SESSION_ID, "some")
                    .with_env(TERRAIN_SELECTED_BIOME, "example_biome")
                    .with_env(TERRAIN_INIT_SCRIPT, compiled_script.to_str().unwrap())
                    .with_expected_output(EXISTING_FPATH)
                    .with_expected_error(false)
                    .with_expected_exit_code(0),
            )
            .and()
            .spawn_command(
                RunCommand::with_exe("/bin/zsh")
                    .with_arg("-i")
                    .with_arg("-s")
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
                    .with_env(TERRAIN_INIT_FN, "terrain-example_biome.zsh")
                    .with_env(TERRAIN_ENABLED, "true")
                    .with_env(TERRAIN_SESSION_ID, "some")
                    .with_env(TERRAIN_SELECTED_BIOME, "example_biome")
                    .with_env(TERRAIN_INIT_SCRIPT, compiled_script.to_str().unwrap())
                    .with_env(FPATH, &fpath)
                    .with_expected_error(false)
                    .with_expected_exit_code(0),
            )
            .successfully();

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(expected_shell_operations),
        );

        copy("./tests/data/terrain.example.toml", context.toml_path())
            .await
            .expect("to copy test terrain.toml");

        let toml_path = current_dir.path().join("terrain.toml");
        let expected_request = AssertExecuteRequest::with()
            .terrain_name("terrainium")
            .biome_name("example_biome")
            .toml_path(toml_path.to_str().unwrap())
            .operation(CONSTRUCTORS)
            .is_activated_as(true)
            .with_expected_reply(
                Any::from_msg(&ExecuteResponse {}).expect("to be converted to any"),
            )
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
                    .with_env(TERRAIN_INIT_FN, "terrain-example_biome.zsh")
                    .with_env(TERRAIN_ENABLED, "true")
                    .with_env(TERRAIN_SESSION_ID, "some")
                    .with_env(TERRAIN_SELECTED_BIOME, "example_biome")
                    .with_env(TERRAIN_INIT_SCRIPT, compiled_script.to_str().unwrap())
                    .with_env(FPATH, &fpath),
            )
            .sent();

        super::handle(
            context,
            None,
            Terrain::example(),
            false,
            Some(expected_request),
        )
        .await
        .expect("no error to be thrown");
    }

    #[tokio::test]
    async fn enter_auto_apply_without_background() {
        let current_dir = tempdir().expect("Couldn't create temp dir");
        let central_dir = tempdir().expect("Couldn't create temp dir");

        let compiled_script = central_dir
            .path()
            .join("scripts")
            .join("terrain-example_biome.zwc");
        const EXISTING_FPATH: &str = "/some/path:/some/path2";
        let fpath = format!("{}:{EXISTING_FPATH}", compiled_script.display());

        let expected_shell_operations = ExpectShell::to()
            .execute(
                RunCommand::with_exe("/bin/zsh")
                    .with_arg("-c")
                    .with_arg("/bin/echo -n $FPATH")
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
                    .with_env(TERRAIN_INIT_FN, "terrain-example_biome.zsh")
                    .with_env(TERRAIN_ENABLED, "true")
                    .with_env(TERRAIN_SESSION_ID, "some")
                    .with_env(TERRAIN_SELECTED_BIOME, "example_biome")
                    .with_env(TERRAIN_INIT_SCRIPT, compiled_script.to_str().unwrap())
                    .with_expected_output(EXISTING_FPATH)
                    .with_expected_error(false)
                    .with_expected_exit_code(0),
            )
            .and()
            .spawn_command(
                RunCommand::with_exe("/bin/zsh")
                    .with_arg("-i")
                    .with_arg("-s")
                    .with_env(TERRAIN_AUTO_APPLY, "enabled")
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
                    .with_env(TERRAIN_INIT_FN, "terrain-example_biome.zsh")
                    .with_env(TERRAIN_ENABLED, "true")
                    .with_env(TERRAIN_SESSION_ID, "some")
                    .with_env(TERRAIN_SELECTED_BIOME, "example_biome")
                    .with_env(TERRAIN_INIT_SCRIPT, compiled_script.to_str().unwrap())
                    .with_env(FPATH, &fpath)
                    .with_expected_error(false)
                    .with_expected_exit_code(0),
            )
            .successfully();

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(expected_shell_operations),
        );

        copy(
            "./tests/data/terrain.example.auto_apply.enabled.toml",
            context.toml_path(),
        )
        .await
        .expect("to copy test terrain.toml");

        let expected_request = AssertExecuteRequest::not_sent();

        let mut terrain = Terrain::example();
        set_auto_apply(&mut terrain, "enable");

        super::handle(context, None, terrain, true, Some(expected_request))
            .await
            .expect("no error to be thrown");
    }

    #[tokio::test]
    async fn enter_auto_apply_with_replace() {
        let current_dir = tempdir().expect("Couldn't create temp dir");
        let central_dir = tempdir().expect("Couldn't create temp dir");

        let compiled_script = central_dir
            .path()
            .join("scripts")
            .join("terrain-example_biome.zwc");
        const EXISTING_FPATH: &str = "/some/path:/some/path2";
        let fpath = format!("{}:{EXISTING_FPATH}", compiled_script.display());

        let expected_shell_operations = ExpectShell::to()
            .execute(
                RunCommand::with_exe("/bin/zsh")
                    .with_arg("-c")
                    .with_arg("/bin/echo -n $FPATH")
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
                    .with_env(TERRAIN_INIT_FN, "terrain-example_biome.zsh")
                    .with_env(TERRAIN_ENABLED, "true")
                    .with_env(TERRAIN_SESSION_ID, "some")
                    .with_env(TERRAIN_SELECTED_BIOME, "example_biome")
                    .with_env(TERRAIN_INIT_SCRIPT, compiled_script.to_str().unwrap())
                    .with_expected_output(EXISTING_FPATH)
                    .with_expected_error(false)
                    .with_expected_exit_code(0),
            )
            .and()
            .spawn_command(
                RunCommand::with_exe("/bin/zsh")
                    .with_arg("-i")
                    .with_arg("-s")
                    .with_cwd(current_dir.path())
                    .with_env(TERRAIN_AUTO_APPLY, "replaced")
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
                    .with_env(TERRAIN_INIT_FN, "terrain-example_biome.zsh")
                    .with_env(TERRAIN_ENABLED, "true")
                    .with_env(TERRAIN_SESSION_ID, "some")
                    .with_env(TERRAIN_SELECTED_BIOME, "example_biome")
                    .with_env(TERRAIN_INIT_SCRIPT, compiled_script.to_str().unwrap())
                    .with_env(FPATH, &fpath)
                    .with_expected_error(false)
                    .with_expected_exit_code(0),
            )
            .successfully();

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(expected_shell_operations),
        );

        copy(
            "./tests/data/terrain.example.auto_apply.replace.toml",
            context.toml_path(),
        )
        .await
        .expect("to copy test terrain.toml");

        let expected_request = AssertExecuteRequest::not_sent();

        let mut terrain = Terrain::example();
        set_auto_apply(&mut terrain, "replace");

        super::handle(context, None, terrain, true, Some(expected_request))
            .await
            .expect("no error to be thrown");
    }

    #[tokio::test]
    async fn enter_auto_apply_with_background() {
        let current_dir = tempdir().expect("Couldn't create temp dir");
        let central_dir = tempdir().expect("Couldn't create temp dir");

        let compiled_script = central_dir
            .path()
            .join("scripts")
            .join("terrain-example_biome.zwc");
        const EXISTING_FPATH: &str = "/some/path:/some/path2";
        let fpath = format!("{}:{EXISTING_FPATH}", compiled_script.display());

        let expected_shell_operations = ExpectShell::to()
            .execute(
                RunCommand::with_exe("/bin/zsh")
                    .with_arg("-c")
                    .with_arg("/bin/echo -n $FPATH")
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
                    .with_env(TERRAIN_INIT_FN, "terrain-example_biome.zsh")
                    .with_env(TERRAIN_ENABLED, "true")
                    .with_env(TERRAIN_SESSION_ID, "some")
                    .with_env(TERRAIN_SELECTED_BIOME, "example_biome")
                    .with_env(TERRAIN_INIT_SCRIPT, compiled_script.to_str().unwrap())
                    .with_expected_output(EXISTING_FPATH)
                    .with_expected_error(false)
                    .with_expected_exit_code(0),
            )
            .and()
            .spawn_command(
                RunCommand::with_exe("/bin/zsh")
                    .with_arg("-i")
                    .with_arg("-s")
                    .with_cwd(current_dir.path())
                    .with_env(TERRAIN_AUTO_APPLY, "background")
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
                    .with_env(TERRAIN_INIT_FN, "terrain-example_biome.zsh")
                    .with_env(TERRAIN_ENABLED, "true")
                    .with_env(TERRAIN_SESSION_ID, "some")
                    .with_env(TERRAIN_SELECTED_BIOME, "example_biome")
                    .with_env(TERRAIN_INIT_SCRIPT, compiled_script.to_str().unwrap())
                    .with_env(FPATH, &fpath)
                    .with_expected_error(false)
                    .with_expected_exit_code(0),
            )
            .successfully();
        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(expected_shell_operations),
        );

        copy(
            "./tests/data/terrain.example.auto_apply.background.toml",
            context.toml_path(),
        )
        .await
        .expect("to copy test terrain.toml");

        let toml_path = current_dir.path().join("terrain.toml");

        let expected_request = AssertExecuteRequest::with()
            .terrain_name("terrainium")
            .biome_name("example_biome")
            .toml_path(toml_path.to_str().unwrap())
            .operation(CONSTRUCTORS)
            .is_activated_as(true)
            .with_expected_reply(
                Any::from_msg(&ExecuteResponse {}).expect("to be converted to any"),
            )
            .with_command(
                RunCommand::with_exe("/bin/bash")
                    .with_arg("-c")
                    .with_arg("$PWD/tests/scripts/print_num_for_10_sec")
                    .with_cwd(current_dir.path())
                    .with_env(TERRAIN_AUTO_APPLY, "background")
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
                    .with_env(TERRAIN_INIT_FN, "terrain-example_biome.zsh")
                    .with_env(TERRAIN_ENABLED, "true")
                    .with_env(TERRAIN_SESSION_ID, "some")
                    .with_env(TERRAIN_SELECTED_BIOME, "example_biome")
                    .with_env(TERRAIN_INIT_SCRIPT, compiled_script.to_str().unwrap())
                    .with_env(FPATH, &fpath),
            )
            .sent();

        let mut terrain = Terrain::example();
        set_auto_apply(&mut terrain, "background");

        super::handle(context, None, terrain, true, Some(expected_request))
            .await
            .expect("no error to be thrown");
    }

    #[tokio::test]
    async fn enter_auto_apply_with_all() {
        let current_dir = tempdir().expect("Couldn't create temp dir");
        let central_dir = tempdir().expect("Couldn't create temp dir");

        let compiled_script = central_dir
            .path()
            .join("scripts")
            .join("terrain-example_biome.zwc");
        const EXISTING_FPATH: &str = "/some/path:/some/path2";
        let fpath = format!("{}:{EXISTING_FPATH}", compiled_script.display());

        let expected_shell_operations = ExpectShell::to()
            .execute(
                RunCommand::with_exe("/bin/zsh")
                    .with_arg("-c")
                    .with_arg("/bin/echo -n $FPATH")
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
                    .with_env(TERRAIN_INIT_FN, "terrain-example_biome.zsh")
                    .with_env(TERRAIN_ENABLED, "true")
                    .with_env(TERRAIN_SESSION_ID, "some")
                    .with_env(TERRAIN_SELECTED_BIOME, "example_biome")
                    .with_env(TERRAIN_INIT_SCRIPT, compiled_script.to_str().unwrap())
                    .with_expected_output(EXISTING_FPATH)
                    .with_expected_error(false)
                    .with_expected_exit_code(0),
            )
            .and()
            .spawn_command(
                RunCommand::with_exe("/bin/zsh")
                    .with_arg("-i")
                    .with_arg("-s")
                    .with_cwd(current_dir.path())
                    .with_env(TERRAIN_AUTO_APPLY, "all")
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
                    .with_env(TERRAIN_INIT_FN, "terrain-example_biome.zsh")
                    .with_env(TERRAIN_ENABLED, "true")
                    .with_env(TERRAIN_SESSION_ID, "some")
                    .with_env(TERRAIN_SELECTED_BIOME, "example_biome")
                    .with_env(TERRAIN_INIT_SCRIPT, compiled_script.to_str().unwrap())
                    .with_env(FPATH, &fpath)
                    .with_expected_error(false)
                    .with_expected_exit_code(0),
            )
            .successfully();
        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(expected_shell_operations),
        );

        copy(
            "./tests/data/terrain.example.auto_apply.all.toml",
            context.toml_path(),
        )
        .await
        .expect("to copy test terrain.toml");

        let toml_path = current_dir.path().join("terrain.toml");

        let expected_request = AssertExecuteRequest::with()
            .terrain_name("terrainium")
            .biome_name("example_biome")
            .toml_path(toml_path.to_str().unwrap())
            .operation(CONSTRUCTORS)
            .is_activated_as(true)
            .with_expected_reply(
                Any::from_msg(&ExecuteResponse {}).expect("to be converted to any"),
            )
            .with_command(
                RunCommand::with_exe("/bin/bash")
                    .with_arg("-c")
                    .with_arg("$PWD/tests/scripts/print_num_for_10_sec")
                    .with_cwd(current_dir.path())
                    .with_env(TERRAIN_AUTO_APPLY, "all")
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
                    .with_env(TERRAIN_INIT_FN, "terrain-example_biome.zsh")
                    .with_env(TERRAIN_ENABLED, "true")
                    .with_env(TERRAIN_SESSION_ID, "some")
                    .with_env(TERRAIN_SELECTED_BIOME, "example_biome")
                    .with_env(TERRAIN_INIT_SCRIPT, compiled_script.to_str().unwrap())
                    .with_env(FPATH, &fpath),
            )
            .sent();

        let mut terrain = Terrain::example();
        set_auto_apply(&mut terrain, "all");

        super::handle(context, None, terrain, true, Some(expected_request))
            .await
            .expect("no error to be thrown");
    }

    #[tokio::test]
    async fn enter_with_no_background_commands_empty() {
        let current_dir = tempdir().expect("Couldn't create temp dir");
        let central_dir = tempdir().expect("Couldn't create temp dir");

        let compiled_script = central_dir.path().join("scripts").join("terrain-none.zwc");
        const EXISTING_FPATH: &str = "/some/path:/some/path2";
        let fpath = format!("{}:{EXISTING_FPATH}", compiled_script.display());

        let expected_shell_operations = ExpectShell::to()
            .execute(
                RunCommand::with_exe("/bin/zsh")
                    .with_arg("-c")
                    .with_arg("/bin/echo -n $FPATH")
                    .with_cwd(current_dir.path())
                    .with_env(TERRAIN_DIR, current_dir.path().to_str().unwrap())
                    .with_env(TERRAIN_INIT_FN, "terrain-none.zsh")
                    .with_env(TERRAIN_ENABLED, "true")
                    .with_env(TERRAIN_SESSION_ID, "some")
                    .with_env(TERRAIN_SELECTED_BIOME, "none")
                    .with_env(TERRAIN_INIT_SCRIPT, compiled_script.to_str().unwrap())
                    .with_expected_output(EXISTING_FPATH)
                    .with_expected_error(false)
                    .with_expected_exit_code(0),
            )
            .and()
            .spawn_command(
                RunCommand::with_exe("/bin/zsh")
                    .with_arg("-i")
                    .with_arg("-s")
                    .with_cwd(current_dir.path())
                    .with_env(TERRAIN_AUTO_APPLY, "all")
                    .with_env(TERRAIN_DIR, current_dir.path().to_str().unwrap())
                    .with_env(TERRAIN_INIT_FN, "terrain-none.zsh")
                    .with_env(TERRAIN_ENABLED, "true")
                    .with_env(TERRAIN_SESSION_ID, "some")
                    .with_env(TERRAIN_SELECTED_BIOME, "none")
                    .with_env(TERRAIN_INIT_SCRIPT, compiled_script.to_str().unwrap())
                    .with_env(FPATH, &fpath)
                    .with_expected_error(false)
                    .with_expected_exit_code(0),
            )
            .successfully();

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(expected_shell_operations),
        );

        copy(
            "./tests/data/terrain.empty.auto_apply.all.toml",
            context.toml_path(),
        )
        .await
        .expect("to copy test terrain.toml");

        let expected_request = AssertExecuteRequest::not_sent();

        let mut terrain = Terrain::default();
        set_auto_apply(&mut terrain, "all");

        super::handle(context, None, terrain, true, Some(expected_request))
            .await
            .expect("no error to be thrown");
    }

    #[tokio::test]
    async fn enter_with_no_background_commands_example() {
        let current_dir = tempdir().expect("Couldn't create temp dir");
        let central_dir = tempdir().expect("Couldn't create temp dir");

        let compiled_script = central_dir
            .path()
            .join("scripts")
            .join("terrain-example_biome.zwc");
        const EXISTING_FPATH: &str = "/some/path:/some/path2";
        let fpath = format!("{}:{EXISTING_FPATH}", compiled_script.display());

        let expected_shell_operations = ExpectShell::to()
            .execute(
                RunCommand::with_exe("/bin/zsh")
                    .with_arg("-c")
                    .with_arg("/bin/echo -n $FPATH")
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
                    .with_env(TERRAIN_INIT_FN, "terrain-example_biome.zsh")
                    .with_env(TERRAIN_ENABLED, "true")
                    .with_env(TERRAIN_SESSION_ID, "some")
                    .with_env(TERRAIN_SELECTED_BIOME, "example_biome")
                    .with_env(TERRAIN_INIT_SCRIPT, compiled_script.to_str().unwrap())
                    .with_expected_output(EXISTING_FPATH)
                    .with_expected_error(false)
                    .with_expected_exit_code(0),
            )
            .and()
            .spawn_command(
                RunCommand::with_exe("/bin/zsh")
                    .with_arg("-i")
                    .with_arg("-s")
                    .with_cwd(current_dir.path())
                    .with_env(TERRAIN_AUTO_APPLY, "background")
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
                    .with_env(TERRAIN_INIT_FN, "terrain-example_biome.zsh")
                    .with_env(TERRAIN_ENABLED, "true")
                    .with_env(TERRAIN_SESSION_ID, "some")
                    .with_env(TERRAIN_SELECTED_BIOME, "example_biome")
                    .with_env(TERRAIN_INIT_SCRIPT, compiled_script.to_str().unwrap())
                    .with_env(FPATH, &fpath)
                    .with_expected_error(false)
                    .with_expected_exit_code(0),
            )
            .successfully();

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Zsh::build(expected_shell_operations),
        );

        copy(
            "./tests/data/terrain.example.without.background.auto_apply.background.toml",
            context.toml_path(),
        )
        .await
        .expect("to copy test terrain.toml");

        let expected_request = AssertExecuteRequest::not_sent();

        let (terrain, _) = Terrain::get_validated_and_fixed_terrain(&context).unwrap();

        super::handle(context, None, terrain, true, Some(expected_request))
            .await
            .expect("no error to be thrown");
    }
}
