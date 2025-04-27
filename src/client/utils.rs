use crate::client::types::client::MockClient;
use crate::common::execute::MockCommandToRun;
use crate::common::types::pb;
use crate::common::types::pb::ExecuteRequest;
use prost_types::Any;
use std::cmp::PartialEq;
use std::collections::BTreeMap;
use std::env::VarError;
use std::fs::read_to_string;
use std::os::unix::prelude::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};

pub const IN_CURRENT_DIR: bool = false;
pub const IN_CENTRAL_DIR: bool = true;

pub const WITH_EMPTY_TERRAIN_TOML: &str = "./tests/data/terrain.empty.toml";
pub const WITH_NONE_BIOME_FOR_EMPTY_TERRAIN_SCRIPT: &str = "./tests/data/terrain-none.empty.zsh";

pub const WITH_EXAMPLE_TERRAIN_TOML_COMMENTS: &str = "./tests/data/terrain.example.comments.toml";
pub const WITH_EXAMPLE_TERRAIN_TOML: &str = "./tests/data/terrain.example.toml";
pub const WITH_NONE_BIOME_FOR_EXAMPLE_SCRIPT: &str = "./tests/data/terrain-none.example.zsh";
pub const WITH_EXAMPLE_BIOME_FOR_EXAMPLE_SCRIPT: &str =
    "./tests/data/terrain-example_biome.example.zsh";

pub const WITHOUT_DEFAULT_BIOME_TOML: &str = "./tests/data/terrain.example.without.default.toml";

pub const WITH_NEW_EXAMPLE_BIOME2_EXAMPLE_TOML: &str =
    "./tests/data/terrain.example.new.example_biome2.toml";
pub const WITH_EXAMPLE_BIOME2_FOR_EXAMPLE_SCRIPT: &str =
    "./tests/data/terrain-example_biome2.example.zsh";

pub const WITH_EXAMPLE_BIOME_UPDATED_EXAMPLE_TOML: &str =
    "./tests/data/terrain.example.example_biome.updated.toml";
pub const WITH_EXAMPLE_BIOME_FOR_UPDATED_EXAMPLE_BIOME_SCRIPT: &str =
    "./tests/data/terrain-example_biome.example.example_biome.updated.zsh";

pub const WITH_NONE_UPDATED_EXAMPLE_TOML: &str = "./tests/data/terrain.example.none.updated.toml";
pub const WITH_NONE_BIOME_FOR_UPDATED_NONE_EXAMPLE_SCRIPT: &str =
    "./tests/data/terrain-none.example.none.updated.zsh";
pub const WITH_EXAMPLE_BIOME_FOR_UPDATED_NONE_EXAMPLE_SCRIPT: &str =
    "./tests/data/terrain-example_biome.example.none.updated.zsh";

pub const WITH_AUTO_APPLY_ENABLED_EXAMPLE_TOML: &str =
    "./tests/data/terrain.example.auto_apply.enabled.toml";

#[derive(Clone)]
pub struct AssertExecuteRequest {
    terrain_name: String,
    biome_name: &'static str,
    toml_path: String,
    is_activate: bool,
    operation: &'static str,
    commands: Vec<RunCommand>,
    reply: Any,
}

impl AssertExecuteRequest {
    pub fn with() -> Self {
        Self {
            terrain_name: "".to_string(),
            biome_name: "",
            is_activate: false,
            operation: "",
            toml_path: "".to_string(),
            commands: vec![],
            reply: Default::default(),
        }
    }

    pub fn not_sent() -> MockClient {
        MockClient::default()
    }

    pub fn terrain_name(mut self, name: &str) -> Self {
        self.terrain_name = name.to_string();
        self
    }

    pub fn biome_name(mut self, name: &'static str) -> Self {
        self.biome_name = name;
        self
    }

    pub fn operation(mut self, name: &'static str) -> Self {
        self.operation = name;
        self
    }

    pub fn with_command(mut self, command: RunCommand) -> Self {
        self.commands.push(command);
        self
    }

    pub fn is_activated_as(mut self, is_active: bool) -> Self {
        self.is_activate = is_active;
        self
    }

    pub fn with_expected_reply(mut self, reply: Any) -> Self {
        self.reply = reply;
        self
    }

    pub fn toml_path(mut self, toml_path: &str) -> Self {
        self.toml_path = toml_path.to_string();
        self
    }

    pub fn sent(self) -> MockClient {
        let mut mock_client = MockClient::default();
        let this = self.clone();
        mock_client
            .expect_write_and_stop()
            .withf(move |execute_request| {
                let request: ExecuteRequest =
                    Any::to_msg(execute_request).expect("request to be converted");
                request.terrain_name == this.terrain_name
                    && request.commands == this.commands
                    && request.biome_name == this.biome_name
                    && request.is_activate == this.is_activate
                    && request.toml_path == this.toml_path
            })
            .return_once(move |_| Ok(()))
            .times(1);

        mock_client
            .expect_read()
            .with()
            .return_once(move || Ok(self.reply.clone()))
            .times(1);

        mock_client
    }
}

#[derive(Clone)]
pub struct RunCommand {
    exe: &'static str,
    args: Vec<String>,
    env_vars: BTreeMap<String, String>,
    cwd: PathBuf,
    exit_code: i32,
    error: bool,
    output: &'static str,
    no_env: bool,
}

impl RunCommand {
    pub fn with_exe(exe: &'static str) -> Self {
        Self {
            exe,
            args: vec![],
            env_vars: Default::default(),
            cwd: Default::default(),
            exit_code: 0,
            error: false,
            output: "",
            no_env: false,
        }
    }

    pub fn with_arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }

    pub fn with_env(mut self, key: &'static str, val: &str) -> Self {
        self.env_vars.insert(key.to_string(), val.to_string());
        self
    }

    pub fn with_no_envs(mut self) -> Self {
        self.no_env = true;
        self
    }

    pub fn with_cwd(mut self, cwd: &Path) -> Self {
        self.cwd = cwd.to_path_buf();
        self
    }

    pub fn with_expected_exit_code(mut self, code: i32) -> Self {
        self.exit_code = code;
        self
    }

    pub fn with_expected_error(mut self, error: bool) -> Self {
        self.error = error;
        self
    }

    pub fn with_expected_output(mut self, output: &'static str) -> Self {
        self.output = output;
        self
    }
}

impl PartialEq<RunCommand> for pb::Command {
    fn eq(&self, other: &RunCommand) -> bool {
        self.exe == other.exe
            && self.args == other.args
            && self.envs == other.env_vars
            && self.cwd == other.cwd.to_str().unwrap()
    }
}

pub struct ExpectShell {
    runner: MockCommandToRun,
}

impl ExpectShell {
    pub fn with(runner: MockCommandToRun) -> Self {
        Self { runner }
    }

    pub fn to() -> Self {
        Self {
            runner: Default::default(),
        }
    }

    pub fn and(self) -> Self {
        self
    }

    pub fn successfully(self) -> MockCommandToRun {
        self.runner
    }

    pub fn execute(mut self, command: RunCommand) -> Self {
        let mut mock_get_output = MockCommandToRun::default();
        mock_get_output
            .expect_set_args()
            .withf(move |args| *args == command.args.clone())
            .return_const(());

        if command.no_env {
            mock_get_output
                .expect_set_envs()
                .withf(|envs| envs.is_none())
                .return_const(());
        } else {
            mock_get_output
                .expect_set_envs()
                .withf(move |envs| *envs == Some(command.env_vars.clone()))
                .return_const(());
        }

        mock_get_output
            .expect_get_output()
            .with()
            .return_once(move || {
                if command.error {
                    Err(anyhow::Error::msg("error"))
                } else {
                    Ok(Output {
                        status: ExitStatus::from_raw(command.exit_code),
                        stdout: Vec::from(command.output),
                        stderr: vec![],
                    })
                }
            });

        self.runner
            .expect_clone()
            .times(1)
            .with()
            .return_once(|| mock_get_output);
        self
    }

    pub fn compile_script_for(self, script_path: &Path, compiled_path: &Path) -> Self {
        self.execute(
            RunCommand::with_exe("/bin/zsh")
                .with_arg("-c")
                .with_arg(&format!(
                    "zcompile -URz {} {}",
                    compiled_path.display(),
                    script_path.display(),
                ))
                .with_no_envs(),
        )
    }

    pub fn compile_terrain_script_for(self, biome_name: &str, central_dir: &Path) -> Self {
        self.compile_script_for(
            &central_dir
                .join("scripts")
                .join(format!("terrain-{biome_name}.zsh")),
            &central_dir
                .join("scripts")
                .join(format!("terrain-{biome_name}.zwc")),
        )
    }

    pub fn spawn_command(mut self, command: RunCommand) -> Self {
        let mut mock_spawn = MockCommandToRun::default();
        mock_spawn
            .expect_set_args()
            .withf(move |args| *args == command.args.clone())
            .return_const(());

        mock_spawn
            .expect_set_envs()
            .withf(move |envs| *envs == Some(command.env_vars.clone()))
            .return_const(());

        mock_spawn.expect_async_spawn().with().return_once(move || {
            if command.error {
                Err(anyhow::Error::msg("error"))
            } else {
                Ok(ExitStatus::from_raw(command.exit_code))
            }
        });

        self.runner
            .expect_clone()
            .times(1)
            .with()
            .return_once(|| mock_spawn);
        self
    }
}

pub struct AssertTerrain<'a> {
    current_dir: &'a Path,
    central_dir: &'a Path,
    older_toml_path: &'a str,
    older_toml: String,
}

impl<'a> AssertTerrain<'a> {
    pub fn with_dirs(current_dir: &'a Path, central_dir: &'a Path) -> Self {
        Self {
            current_dir,
            central_dir,
            older_toml_path: "",
            older_toml: "".to_string(),
        }
    }

    pub fn with_dirs_and_existing(
        current_dir: &'a Path,
        central_dir: &'a Path,
        existing_terrain: &'a str,
    ) -> Self {
        let older_toml = read_to_string(existing_terrain).expect("to be read");
        Self {
            current_dir,
            central_dir,
            older_toml_path: existing_terrain,
            older_toml,
        }
    }

    pub fn scripts_dir_was_created(self) -> Self {
        assert!(
            self.central_dir.join("scripts").exists(),
            "failed to find scripts dir"
        );
        self
    }

    pub fn central_dir_is_created(self) -> Self {
        assert!(self.central_dir.exists(), "failed to find central dir");
        self
    }

    pub fn was_initialized(self, in_central: bool, mode: &'static str) -> Self {
        let toml = if in_central {
            self.central_dir
        } else {
            self.current_dir
        }
        .join("terrain.toml");

        assert!(toml.exists(), "failed to find terrain.toml");
        assert_eq!(
            read_to_string(&toml).expect("to find terrain.toml"),
            read_to_string(mode).expect("to find test terrain.toml"),
            "failed to validate that terrain.toml was created for in_central {in_central} for test terrain: {mode}",
        );

        self
    }

    pub fn script_was_created_for(self, biome_name: &str) -> Self {
        let script = self
            .central_dir
            .join("scripts")
            .join(format!("terrain-{biome_name}.zsh"));

        assert!(
            script.exists(),
            "failed to find script for biome {biome_name}",
        );

        self
    }

    pub fn was_updated(self, in_central: bool, new_toml_path: &'static str) -> Self {
        let toml = if in_central {
            self.central_dir
        } else {
            self.current_dir
        }
        .join("terrain.toml");

        assert!(toml.exists(), "failed to find terrain.toml");

        let new_toml_contents = read_to_string(&toml).expect("to find terrain.toml");

        assert_ne!(self.older_toml_path, new_toml_path);
        assert_ne!(self.older_toml, new_toml_contents);
        assert_eq!(
            new_toml_contents,
            read_to_string(new_toml_path).expect("to find test terrain.toml"),
            "failed to validate terrain.toml was created for in_central {in_central} for test terrain: {new_toml_path}",
        );

        self
    }

    pub fn with_backup(self, in_central: bool) -> Self {
        let backup = if in_central {
            self.central_dir
        } else {
            self.current_dir
        }
        .join("terrain.toml.bkp");

        assert!(backup.exists(), "failed to find terrain.toml");
        assert_eq!(
            self.older_toml,
            read_to_string(backup).expect("to find test terrain.toml"),
            "failed to check terrain.toml.bkp was created for in_central {in_central} for test terrain: {}",
            self.older_toml_path
        );

        self
    }

    pub fn was_not_updated(self, in_central: bool) -> Self {
        let toml = if in_central {
            self.central_dir
        } else {
            self.current_dir
        }
        .join("terrain.toml");

        assert!(toml.exists(), "failed to find terrain.toml");

        let new_toml_contents = read_to_string(&toml).expect("to find terrain.toml");

        assert_eq!(
            new_toml_contents,
            read_to_string(self.older_toml_path).expect("to find test terrain.toml"),
            "failed to check terrain.toml was created for in_central {in_central} for test terrain: {}",
            self.older_toml_path
        );

        self
    }
}

pub fn set_env_var(key: String, value: Option<String>) -> Result<String, VarError> {
    // FIX: the tests run in parallel so setting same env var will cause tests to fail
    // as env var is not reset yet
    let orig_env = std::env::var(&key);
    if let Some(val) = value {
        std::env::set_var(&key, val);
    } else {
        std::env::remove_var(&key);
    }

    orig_env
}

pub fn restore_env_var(key: String, orig_env: anyhow::Result<String, VarError>) {
    // FIX: the tests run in parallel so restoring env vars won't help if vars have same key
    if let Ok(orig_var) = orig_env {
        std::env::set_var(&key, &orig_var);
        assert!(std::env::var(&key).is_ok());
        assert_eq!(orig_var, std::env::var(&key).expect("var to be present"));
    } else {
        std::env::remove_var(&key);
        assert!(std::env::var(&key).is_err());
    }
}
