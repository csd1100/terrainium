use crate::client::test_utils::assertions::executor::{AssertExecutor, ExpectedCommand};
use crate::common::execute::MockExecutor;
use crate::common::types::command::Command;
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

    pub fn compile_script_for(self, script_path: &Path, compiled_path: &Path) -> Self {
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
            exit_code: 0,
            should_error: false,
            output: "",
        };

        Self {
            executor: AssertExecutor::with(self.executor)
                .get_output_for(expected)
                .successfully(),
            cwd: self.cwd,
        }
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

    pub fn successfully(self) -> MockExecutor {
        self.executor
    }
}
