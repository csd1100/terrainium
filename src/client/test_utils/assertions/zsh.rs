use crate::client::test_utils::assertions::command::RunCommand;
use crate::common::types::command::MockCommand;
use std::os::unix::prelude::ExitStatusExt;
use std::path::Path;
use std::process::{ExitStatus, Output};

pub struct ExpectZSH {
    runner: MockCommand,
}

impl ExpectZSH {
    pub fn with(runner: MockCommand) -> Self {
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

    pub fn successfully(self) -> MockCommand {
        self.runner
    }

    pub fn execute(mut self, command: RunCommand) -> Self {
        let mut mock_get_output = MockCommand::default();
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
        let mut mock_spawn = MockCommand::default();
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
