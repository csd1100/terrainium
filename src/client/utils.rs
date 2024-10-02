#[cfg(test)]
pub(crate) mod test {
    use crate::common::execute::MockRun;
    use std::os::unix::prelude::ExitStatusExt;
    use std::path::{Path, PathBuf};
    use std::process::{ExitStatus, Output};

    pub(crate) fn setup_with_expectations(
        mut mock: MockRun,
        mock_with_expectations: MockRun,
    ) -> MockRun {
        mock.expect_clone()
            .with()
            .times(1)
            .return_once(|| mock_with_expectations);
        mock
    }

    pub(crate) fn compile_expectations(central_dir: PathBuf, biome_name: String) -> MockRun {
        let compile_script_path = compiled_script_path(central_dir.as_path(), &biome_name);
        let script_path = script_path(central_dir.as_path(), &biome_name);

        let mut mock_runner = MockRun::default();
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
        let mut script_path = scripts_dir(central_dir).clone();
        script_path.push(format!("terrain-{}.zsh", biome_name));
        script_path
    }

    pub(crate) fn compiled_script_path(central_dir: &Path, biome_name: &String) -> PathBuf {
        let mut compile_script_path = scripts_dir(central_dir).clone();
        compile_script_path.push(format!("terrain-{}.zwc", biome_name));
        compile_script_path
    }

    pub(crate) fn scripts_dir(central_dir: &Path) -> PathBuf {
        let mut scripts_dir: PathBuf = central_dir.into();
        scripts_dir.push("scripts");
        scripts_dir
    }
}