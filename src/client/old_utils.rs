#[cfg(test)]
pub(crate) mod test {
    use crate::common::execute::MockCommandToRun;
    use std::os::unix::prelude::ExitStatusExt;
    use std::path::{Path, PathBuf};
    use std::process::{ExitStatus, Output};

    pub(crate) fn setup_command_runner_mock_with_expectations(
        mut mock: MockCommandToRun,
        mock_with_expectations: MockCommandToRun,
    ) -> MockCommandToRun {
        mock.expect_clone()
            .with()
            .times(1)
            .return_once(|| mock_with_expectations);
        mock
    }

    pub(crate) fn compile_expectations(
        central_dir: PathBuf,
        biome_name: String,
    ) -> MockCommandToRun {
        let compile_script_path = compiled_script_path(central_dir.as_path(), &biome_name);
        let script_path = script_path(central_dir.as_path(), &biome_name);

        let mut mock_runner = MockCommandToRun::default();
        let args = vec![
            "-c".to_string(),
            format!(
                "zcompile -URz {} {}",
                compile_script_path.to_string_lossy(),
                script_path.to_string_lossy()
            ),
        ];

        mock_runner
            .expect_set_args()
            .withf(move |actual_args| *actual_args == args)
            .returning(|_| ());

        mock_runner
            .expect_set_envs()
            .withf(move |envs| envs.is_none())
            .times(1)
            .returning(|_| ());

        mock_runner
            .expect_get_output()
            .with()
            .times(1)
            .returning(|| {
                Ok(Output {
                    status: ExitStatus::from_raw(0),
                    stdout: Vec::<u8>::from("/tmp/test/path"),
                    stderr: Vec::<u8>::new(),
                })
            });
        mock_runner
    }

    pub(crate) fn script_path(central_dir: &Path, biome_name: &String) -> PathBuf {
        scripts_dir(central_dir)
            .clone()
            .join(format!("terrain-{}.zsh", biome_name))
    }

    pub(crate) fn compiled_script_path(central_dir: &Path, biome_name: &String) -> PathBuf {
        scripts_dir(central_dir)
            .clone()
            .join(format!("terrain-{}.zwc", biome_name))
    }

    pub(crate) fn scripts_dir(central_dir: &Path) -> PathBuf {
        central_dir.join("scripts")
    }
}
