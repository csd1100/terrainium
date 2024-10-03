use crate::client::args::{option_string_from, BiomeArg};
use crate::client::handlers::background;
use crate::client::shell::Shell;
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use crate::common::constants::CONSTRUCTORS;
use anyhow::{Context as AnyhowContext, Result};
use tokio::fs::read_to_string;

pub async fn handle(context: &mut Context, biome_arg: Option<BiomeArg>) -> Result<()> {
    let terrain = Terrain::from_toml(
        read_to_string(context.toml_path()?)
            .await
            .context("failed to read terrain.toml")?,
    )
    .expect("failed to parse terrain from toml");

    let biome = option_string_from(&biome_arg);
    let (selected_name, _) = terrain.select_biome(&biome)?;

    let mut envs = terrain.merged_envs(&biome)?;
    envs.append(&mut context.terrainium_envs().clone());
    envs.append(
        &mut context
            .shell()
            .generate_envs(context, selected_name.to_string())?
            .clone(),
    );

    background::handle(
        context,
        CONSTRUCTORS,
        biome_arg,
        Terrain::merged_constructors,
        Some(envs.clone()),
    )
    .await
    .context("failed to send background constructors to daemon")?;

    context
        .shell()
        .spawn(envs)
        .await
        .context("failed to spawn shell")?;

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::client::shell::Zsh;
    use crate::client::types::client::MockClient;
    use crate::client::types::context::Context;
    use crate::common::constants::{
        FPATH, TERRAINIUM_ENABLED, TERRAINIUM_SESSION_ID, TERRAIN_ACTIVATION_TIMESTAMP,
        TERRAIN_DIR, TERRAIN_INIT_FN, TERRAIN_INIT_SCRIPT, TERRAIN_SELECTED_BIOME,
    };
    use crate::common::execute::MockCommandToRun;
    use crate::common::types::pb::{Command, ExecuteRequest, ExecuteResponse, Operation};
    use prost_types::Any;
    use serial_test::serial;
    use std::collections::BTreeMap;
    use std::os::unix::process::ExitStatusExt;
    use std::path::PathBuf;
    use std::process::{ExitStatus, Output};
    use tempfile::tempdir;
    use tokio::fs::copy;

    #[serial]
    #[tokio::test]
    async fn enter_default() {
        let current_dir = tempdir().expect("Couldn't create temp dir");
        let central_dir = tempdir().expect("Couldn't create temp dir");

        let mut compiled_script = central_dir.path().join("scripts");
        compiled_script.push("terrain-example_biome.zwc");

        let mut script = central_dir.path().join("scripts");
        script.push("terrain-example_biome.zsh");

        let mut expected_envs = BTreeMap::<String, String>::new();
        expected_envs.insert("EDITOR".to_string(), "nvim".to_string());
        expected_envs.insert("PAGER".to_string(), "less".to_string());
        expected_envs.insert(
            TERRAIN_DIR.to_string(),
            current_dir.path().display().to_string(),
        );
        expected_envs.insert(
            TERRAIN_INIT_SCRIPT.to_string(),
            compiled_script.display().to_string(),
        );
        expected_envs.insert(
            TERRAIN_INIT_FN.to_string(),
            "terrain-example_biome.zsh".to_string(),
        );
        expected_envs.insert(TERRAINIUM_ENABLED.to_string(), "true".to_string());
        expected_envs.insert(TERRAINIUM_SESSION_ID.to_string(), "some".to_string());
        expected_envs.insert(
            TERRAIN_SELECTED_BIOME.to_string(),
            "example_biome".to_string(),
        );

        const EXISTING_FPATH: &str = "/some/path:/some/path2";
        expected_envs.insert(
            FPATH.to_string(),
            format!("{}:{}", compiled_script.display(), EXISTING_FPATH),
        );

        let mut spawn = MockCommandToRun::default();
        spawn
            .expect_set_args()
            .withf(|args| *args == vec!["-i", "-s"])
            .times(1)
            .return_once(|_| ());

        let spawn_envs = expected_envs.clone();
        spawn
            .expect_set_envs()
            .withf(move |envs| {
                let envs = envs.clone().expect("env vars to be passed");
                envs.iter()
                    .filter(|(var, _val)| *var != TERRAIN_ACTIVATION_TIMESTAMP)
                    .all(|(var, val)| val == &spawn_envs[var])
            })
            .times(1)
            .return_once(|_| ());

        spawn
            .expect_async_spawn()
            .times(1)
            .return_once(|| Ok(ExitStatus::from_raw(0)));

        let mut fpath = MockCommandToRun::default();
        fpath
            .expect_set_args()
            .withf(|args| *args == vec!["-c", "/bin/echo -n $FPATH"])
            .return_once(|_| ());
        fpath.expect_get_output().times(1).return_once(|| {
            Ok(Output {
                status: ExitStatus::from_raw(0),
                stdout: Vec::from(EXISTING_FPATH),
                stderr: vec![],
            })
        });

        let mut shell_runner = MockCommandToRun::default();
        shell_runner.expect_clone().times(1).return_once(|| fpath);
        shell_runner.expect_clone().times(1).return_once(|| spawn);
        let shell = Zsh::build(shell_runner);

        let toml_path = current_dir
            .path()
            .join("terrain.toml")
            .display()
            .to_string();
        let current_dir_path: PathBuf = current_dir.path().into();
        let mut mocket = MockClient::default();
        mocket
            .expect_write_and_stop()
            .withf(move |actual: &Any| {
                let terrain_name = current_dir_path
                    .file_name()
                    .expect("to be present")
                    .to_str()
                    .expect("converted to string")
                    .to_string();

                let commands = vec![Command {
                    exe: "/bin/bash".to_string(),
                    args: vec![
                        "-c".to_string(),
                        "$PWD/tests/scripts/print_num_for_10_sec".to_string(),
                    ],
                    envs: expected_envs.clone(),
                }];

                let actual: ExecuteRequest =
                    Any::to_msg(actual).expect("failed to convert to Activate request");

                actual.terrain_name == terrain_name
                    && !actual.session_id.is_empty()
                    && actual.biome_name == "example_biome"
                    && actual.toml_path == toml_path
                    && actual.is_activate
                    && actual.commands == commands
                    && actual.operation == i32::from(Operation::Constructors)
            })
            .times(1)
            .return_once(move |_| Ok(()));

        mocket.expect_read().with().times(1).return_once(|| {
            Ok(Any::from_msg(&ExecuteResponse {}).expect("to be converted to any"))
        });

        let mut context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            shell,
            Some(mocket),
        );

        copy(
            "./tests/data/terrain.example.toml",
            context.new_toml_path(false),
        )
        .await
        .expect("to copy test terrain.toml");

        super::handle(&mut context, None)
            .await
            .expect("no error to be thrown");
    }
}
