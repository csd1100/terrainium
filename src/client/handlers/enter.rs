use crate::client::args::{option_string_from, BiomeArg};
use crate::client::handlers::background;
use crate::client::shell::Shell;
#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use crate::common::constants::{CONSTRUCTORS, TERRAIN_AUTO_APPLY};
use anyhow::{anyhow, Context as AnyhowContext, Result};
use tokio::fs::read_to_string;

pub async fn handle(
    context: Context,
    biome_arg: Option<BiomeArg>,
    auto_apply: bool,
    client: Option<Client>,
) -> Result<()> {
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

    if client.is_none() || (client.is_some() && auto_apply && !terrain.auto_apply().is_background())
    {
        context
            .shell()
            .spawn(envs)
            .await
            .context("failed to enter terrain environment")?;
    } else {
        let result = tokio::try_join!(
            background::handle(
                &context,
                client.unwrap(),
                CONSTRUCTORS,
                biome_arg,
                Some(envs.clone()),
            ),
            context.shell().spawn(envs)
        );

        if let Err(err) = result {
            return Err(anyhow!("failed to enter the terrain: {}", err));
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::client::shell::Zsh;
    use crate::client::types::client::MockClient;
    use crate::client::types::context::Context;
    use crate::common::constants::{
        FPATH, TERRAINIUM_EXECUTABLE, TERRAIN_ACTIVATION_TIMESTAMP, TERRAIN_AUTO_APPLY,
        TERRAIN_DIR, TERRAIN_ENABLED, TERRAIN_INIT_FN, TERRAIN_INIT_SCRIPT, TERRAIN_SELECTED_BIOME,
        TERRAIN_SESSION_ID,
    };
    use crate::common::execute::MockCommandToRun;
    use crate::common::types::pb::{Command, ExecuteRequest, ExecuteResponse, Operation};
    use prost_types::Any;
    use serial_test::serial;
    use std::collections::BTreeMap;
    use std::env;
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
        expected_envs.insert(TERRAIN_ENABLED.to_string(), "true".to_string());
        expected_envs.insert(TERRAIN_SESSION_ID.to_string(), "some".to_string());
        expected_envs.insert(
            TERRAIN_SELECTED_BIOME.to_string(),
            "example_biome".to_string(),
        );
        let exe = env::args().next().unwrap();
        expected_envs.insert(TERRAINIUM_EXECUTABLE.to_string(), exe);

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

        let context = Context::build(current_dir.path().into(), central_dir.path().into(), shell);

        copy(
            "./tests/data/terrain.example.toml",
            context.new_toml_path(false),
        )
        .await
        .expect("to copy test terrain.toml");

        super::handle(context, None, false, Some(mocket))
            .await
            .expect("no error to be thrown");
    }

    #[serial]
    #[tokio::test]
    async fn enter_auto_apply_without_background() {
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
        expected_envs.insert(TERRAIN_ENABLED.to_string(), "true".to_string());
        expected_envs.insert(TERRAIN_SESSION_ID.to_string(), "some".to_string());
        expected_envs.insert(
            TERRAIN_SELECTED_BIOME.to_string(),
            "example_biome".to_string(),
        );
        let exe = env::args().next().unwrap();
        expected_envs.insert(TERRAINIUM_EXECUTABLE.to_string(), exe);
        expected_envs.insert(TERRAIN_AUTO_APPLY.to_string(), "enabled".to_string());

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

        let context = Context::build(current_dir.path().into(), central_dir.path().into(), shell);

        copy(
            "./tests/data/terrain.example.auto_apply.enabled.toml",
            context.new_toml_path(false),
        )
        .await
        .expect("to copy test terrain.toml");

        super::handle(context, None, true, Some(MockClient::default()))
            .await
            .expect("no error to be thrown");
    }

    #[serial]
    #[tokio::test]
    async fn enter_auto_apply_with_all() {
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
        expected_envs.insert(TERRAIN_ENABLED.to_string(), "true".to_string());
        expected_envs.insert(TERRAIN_AUTO_APPLY.to_string(), "all".to_string());
        expected_envs.insert(TERRAIN_SESSION_ID.to_string(), "some".to_string());
        expected_envs.insert(
            TERRAIN_SELECTED_BIOME.to_string(),
            "example_biome".to_string(),
        );
        let exe = env::args().next().unwrap();
        expected_envs.insert(TERRAINIUM_EXECUTABLE.to_string(), exe);

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

        let context = Context::build(current_dir.path().into(), central_dir.path().into(), shell);

        copy(
            "./tests/data/terrain.example.auto_apply.all.toml",
            context.new_toml_path(false),
        )
        .await
        .expect("to copy test terrain.toml");

        super::handle(context, None, true, Some(mocket))
            .await
            .expect("no error to be thrown");
    }

    #[serial]
    #[tokio::test]
    async fn enter_auto_apply_with_background() {
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
        expected_envs.insert(TERRAIN_ENABLED.to_string(), "true".to_string());
        expected_envs.insert(TERRAIN_AUTO_APPLY.to_string(), "background".to_string());
        expected_envs.insert(TERRAIN_SESSION_ID.to_string(), "some".to_string());
        expected_envs.insert(
            TERRAIN_SELECTED_BIOME.to_string(),
            "example_biome".to_string(),
        );
        let exe = env::args().next().unwrap();
        expected_envs.insert(TERRAINIUM_EXECUTABLE.to_string(), exe);

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

        let context = Context::build(current_dir.path().into(), central_dir.path().into(), shell);

        copy(
            "./tests/data/terrain.example.auto_apply.background.toml",
            context.new_toml_path(false),
        )
        .await
        .expect("to copy test terrain.toml");

        super::handle(context, None, true, Some(mocket))
            .await
            .expect("no error to be thrown");
    }

    #[serial]
    #[tokio::test]
    async fn enter_auto_apply_with_replace() {
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
        expected_envs.insert(TERRAIN_ENABLED.to_string(), "true".to_string());
        expected_envs.insert(TERRAIN_SESSION_ID.to_string(), "some".to_string());
        expected_envs.insert(
            TERRAIN_SELECTED_BIOME.to_string(),
            "example_biome".to_string(),
        );
        let exe = env::args().next().unwrap();
        expected_envs.insert(TERRAINIUM_EXECUTABLE.to_string(), exe);
        expected_envs.insert(TERRAIN_AUTO_APPLY.to_string(), "replaced".to_string());

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

        let context = Context::build(current_dir.path().into(), central_dir.path().into(), shell);

        copy(
            "./tests/data/terrain.example.auto_apply.replace.toml",
            context.new_toml_path(false),
        )
        .await
        .expect("to copy test terrain.toml");

        super::handle(context, None, true, Some(MockClient::default()))
            .await
            .expect("no error to be thrown");
    }
}
