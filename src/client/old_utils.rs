#[cfg(test)]
pub(crate) mod test {
    use crate::client::types::client::MockClient;
    use crate::common::constants::TERRAINIUM_EXECUTABLE;
    use crate::common::execute::MockCommandToRun;
    use crate::common::types::pb::{Command, ExecuteRequest, ExecuteResponse, Operation};
    use prost_types::Any;
    use std::collections::BTreeMap;
    use std::os::unix::prelude::ExitStatusExt;
    use std::path::{Path, PathBuf};
    use std::process::{ExitStatus, Output};

    pub(crate) fn mock_client_with_successful_constructor_execution_request(
        current_dir_path: PathBuf,
        terrain_toml: PathBuf,
    ) -> MockClient {
        mock_client_with_execution_request(current_dir_path, terrain_toml, false)
    }

    pub(crate) fn mock_client_with_execution_request(
        current_dir_path: PathBuf,
        terrain_toml: PathBuf,
        is_activate: bool,
    ) -> MockClient {
        let mut mock_client = MockClient::default();
        mock_client
            .expect_write_and_stop()
            .withf(move |actual: &Any| {
                let mut envs: BTreeMap<String, String> = BTreeMap::new();
                envs.insert("EDITOR".to_string(), "nvim".to_string());
                envs.insert("PAGER".to_string(), "less".to_string());
                envs.insert(
                    "TERRAIN_DIR".to_string(),
                    current_dir_path.to_str().unwrap().to_string(),
                );
                envs.insert("TERRAIN_ENABLED".to_string(), "true".to_string());

                let exe = std::env::args().next().unwrap();
                envs.insert(TERRAINIUM_EXECUTABLE.to_string(), exe);

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
                    envs,
                }];

                let actual: ExecuteRequest =
                    Any::to_msg(actual).expect("failed to convert to Activate request");

                actual.terrain_name == terrain_name
                    && actual.session_id.is_empty()
                    && actual.biome_name == "example_biome"
                    && actual.toml_path == terrain_toml.display().to_string()
                    && actual.is_activate == is_activate
                    && actual.commands == commands
                    && actual.operation == i32::from(Operation::Constructors)
            })
            .times(1)
            .return_once(move |_| Ok(()));

        mock_client.expect_read().with().times(1).return_once(|| {
            Ok(Any::from_msg(&ExecuteResponse {}).expect("to be converted to any"))
        });

        mock_client
    }

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
