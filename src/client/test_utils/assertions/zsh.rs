use crate::client::test_utils::assertions::executor::{AssertExecutor, ExpectedCommand};
use crate::common::execute::MockExecutor;
use crate::common::test_utils::TEST_FPATH;
use crate::common::types::command::Command;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const ZSH: &str = "/bin/zsh";

pub struct ExpectZSH {
    executor: MockExecutor,
    cwd: PathBuf,
}

impl ExpectZSH {
    pub fn with(executor: MockExecutor, cwd: &Path) -> Self {
        Self {
            executor,
            cwd: cwd.into(),
        }
    }

    fn compile_script(
        self,
        script_path: &Path,
        compiled_path: &Path,
        should_fail_to_execute: bool,
        exit_code: i32,
        output: String,
    ) -> Self {
        let command = Command::new(
            ZSH.to_string(),
            vec![
                "-c".to_string(),
                format!(
                    "zcompile -URz {} {}",
                    compiled_path.display(),
                    script_path.display(),
                ),
            ],
            Some(self.cwd.clone()),
        );
        let expected = ExpectedCommand {
            command,
            exit_code,
            should_fail_to_execute,
            output,
        };

        Self {
            executor: AssertExecutor::with(self.executor)
                .get_output_for(None, expected, 1)
                .successfully(),
            cwd: self.cwd,
        }
    }

    pub fn compile_script_successfully_for(self, script_path: &Path, compiled_path: &Path) -> Self {
        self.compile_script(script_path, compiled_path, false, 0, String::new())
    }

    pub fn compile_script_with_non_zero_exit_code(
        self,
        script_path: &Path,
        compiled_path: &Path,
    ) -> Self {
        self.compile_script(
            script_path,
            compiled_path,
            false,
            1,
            "some error while compiling".to_string(),
        )
    }

    pub fn compile_terrain_script_for(self, biome_name: &str, central_dir: &Path) -> Self {
        self.compile_script_successfully_for(
            &central_dir
                .join("scripts")
                .join(format!("terrain-{biome_name}.zsh")),
            &central_dir
                .join("scripts")
                .join(format!("terrain-{biome_name}.zwc")),
        )
    }

    pub fn get_fpath(self) -> Self {
        let ExpectZSH { executor, cwd } = self;
        let executor = AssertExecutor::with(executor)
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(
                        "/bin/zsh".to_string(),
                        vec!["-c".to_string(), "/bin/echo -n $FPATH".to_string()],
                        Some(cwd.to_path_buf()),
                    ),
                    exit_code: 0,
                    should_fail_to_execute: false,
                    output: TEST_FPATH.to_string(),
                },
                1,
            )
            .successfully();
        Self { executor, cwd }
    }

    pub fn spawn_shell(
        self,
        envs: BTreeMap<String, String>,
        exit_code: i32,
        should_fail_to_execute: bool,
        error_message: String,
    ) -> Self {
        let ExpectZSH { executor, cwd } = self;
        let executor = AssertExecutor::with(executor)
            .async_spawn(
                Some(Arc::new(envs)),
                ExpectedCommand {
                    command: Command::new(
                        "/bin/zsh".to_string(),
                        vec!["-i".to_string(), "-s".to_string()],
                        Some(cwd.to_path_buf()),
                    ),
                    exit_code,
                    should_fail_to_execute,
                    output: error_message,
                },
                1,
            )
            .successfully();
        Self { executor, cwd }
    }

    pub fn successfully(self) -> MockExecutor {
        self.executor
    }
}
