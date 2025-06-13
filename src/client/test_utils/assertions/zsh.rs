use crate::client::test_utils::assertions::executor::{AssertExecutor, ExpectedCommand};
use crate::common::execute::MockExecutor;
use crate::common::types::command::Command;
use crate::common::types::test_utils::TEST_FPATH;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const ZSH: &str = "/bin/zsh";

pub struct ExpectZSH {
    executor: MockExecutor,
    cwd: PathBuf,
}

impl ExpectZSH {
    pub fn with(executor: MockExecutor, cwd: PathBuf) -> Self {
        Self { executor, cwd }
    }

    fn compile_script(
        self,
        script_path: &Path,
        compiled_path: &Path,
        should_error: bool,
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
            None,
            Some(self.cwd.clone()),
        );
        let expected = ExpectedCommand {
            command,
            exit_code,
            should_error,
            output,
        };

        Self {
            executor: AssertExecutor::with(self.executor)
                .get_output_for(expected)
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
            true,
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
            .get_output_for(ExpectedCommand {
                command: Command::new(
                    "/bin/zsh".to_string(),
                    vec!["-c".to_string(), "/bin/echo -n $FPATH".to_string()],
                    None,
                    Some(cwd.clone().to_path_buf()),
                ),
                exit_code: 0,
                should_error: false,
                output: TEST_FPATH.to_string(),
            })
            .successfully();
        Self { executor, cwd }
    }

    pub fn spawn_shell(
        self,
        envs: BTreeMap<String, String>,
        exit_code: i32,
        should_error: bool,
        error_message: String,
    ) -> Self {
        let ExpectZSH { executor, cwd } = self;
        let executor = AssertExecutor::with(executor)
            .async_spawn(ExpectedCommand {
                command: Command::new(
                    "/bin/zsh".to_string(),
                    vec!["-i".to_string(), "-s".to_string()],
                    Some(envs),
                    Some(cwd.clone().to_path_buf()),
                ),
                exit_code,
                should_error,
                output: error_message,
            })
            .successfully();
        Self { executor, cwd }
    }

    pub fn successfully(self) -> MockExecutor {
        self.executor
    }
}
