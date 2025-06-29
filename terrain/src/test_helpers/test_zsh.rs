use std::path::{Path, PathBuf};

use terrainium_lib::command::Command;
use terrainium_lib::executor::MockExecute;
use terrainium_lib::test_utils::execute::ExpectExecutor;

const ZSH_BIN: &str = "/bin/zsh";

pub const ZSH_INTEGRATION_SCRIPT: &str = "../tests/data/terrainium_init.zsh";
pub const ZSH_INTEGRATION_SCRIPT_RELEASE: &str = "../tests/data/terrainium_init-release.zsh";

pub struct ExpectZSH {
    executor: MockExecute,
    cwd: PathBuf,
}

impl ExpectZSH {
    /// command that will compile zsh script using zcompile
    fn compile_command(&self, compiled_path: &Path, script_path: &Path) -> Command {
        Command::new(
            ZSH_BIN.to_string(),
            vec![
                "-c".to_string(),
                format!(
                    "zcompile -URz {} {}",
                    compiled_path.display(),
                    script_path.display(),
                ),
            ],
            Some(self.cwd.clone()),
        )
    }

    pub fn to(cwd: &Path) -> Self {
        Self {
            executor: MockExecute::new(),
            cwd: cwd.into(),
        }
    }

    /// compile script using zcompile script
    pub fn compile_script_successfully_for(
        self,
        script_path: &Path,
        compiled_path: &Path,
    ) -> MockExecute {
        let command = self.compile_command(compiled_path, script_path);
        ExpectExecutor::with(self.executor).successfully_get_output_for(
            None,
            command,
            "".to_string(),
            1,
        )
    }
}
