use anyhow::Result;
use std::process::{ExitStatus, Output};
pub trait Execute {
    fn get_output(self) -> Result<Output>;
    fn wait(self) -> Result<ExitStatus>;
    fn async_get_output(self) -> impl std::future::Future<Output = Result<Output>> + Send;
    fn async_wait(
        self,
        log_path: &str,
    ) -> impl std::future::Future<Output = Result<ExitStatus>> + Send;
    fn async_spawn(self) -> impl std::future::Future<Output = Result<ExitStatus>> + Send;
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::client::test_utils;
    use crate::common::execute::Execute;
    use crate::common::types::command::Command;
    use anyhow::Result;
    use std::collections::BTreeMap;

    #[test]
    fn test_spawn_and_get_output_without_envs() -> Result<()> {
        let test_var = "TEST_VAR".to_string();
        let orig_env = test_utils::set_env_var(test_var.clone(), Some("TEST_VALUE".to_string()));

        let run = Command::new(
            "/bin/bash".to_string(),
            vec!["-c".to_string(), "echo $TEST_VAR".to_string()],
            None,
            Some(std::env::current_dir()?),
        );

        let output = run.get_output().expect("not to fail");

        assert_eq!(
            "TEST_VALUE\n",
            String::from_utf8(output.stdout).expect("convert to ascii")
        );

        test_utils::restore_env_var(test_var.clone(), orig_env);

        Ok(())
    }

    #[test]
    fn test_spawn_and_get_output_with_envs() -> Result<()> {
        let test_var1: String = "TEST_VAR1".to_string();
        let test_var2 = "TEST_VAR2".to_string();

        let orig_env1 = test_utils::set_env_var(test_var1.clone(), Some("OLD_VALUE1".to_string()));
        let orig_env2 = test_utils::set_env_var(test_var2.clone(), Some("OLD_VALUE2".to_string()));

        let mut envs: BTreeMap<String, String> = BTreeMap::new();
        envs.insert(test_var1.clone(), "NEW_VALUE1".to_string());

        let run = Command::new(
            "/bin/bash".to_string(),
            vec![
                "-c".to_string(),
                "echo \"$TEST_VAR1\n$TEST_VAR2\"".to_string(),
            ],
            Some(envs),
            Some(std::env::current_dir()?),
        );

        let output = run.get_output().expect("not to fail");

        assert_eq!(
            "NEW_VALUE1\nOLD_VALUE2\n",
            String::from_utf8(output.stdout).expect("convert to ascii")
        );

        test_utils::restore_env_var(test_var1, orig_env1);
        test_utils::restore_env_var(test_var2, orig_env2);

        Ok(())
    }

    #[test]
    fn test_run_set_args_and_envs() -> Result<()> {
        let test_var = "TEST_VAR".to_string();

        let mut envs: BTreeMap<String, String> = BTreeMap::new();
        envs.insert(test_var.clone(), "TEST_VALUE".to_string());

        let args: Vec<String> = vec!["-c".to_string(), "echo \"$TEST_VAR\"".to_string()];

        let mut run = Command::new(
            "/bin/bash".to_string(),
            vec![],
            None,
            Some(std::env::current_dir()?),
        );
        run.set_envs(Some(envs));
        run.set_args(args);

        let output = run.get_output().expect("not to fail");

        assert_eq!(
            "TEST_VALUE\n",
            String::from_utf8(output.stdout).expect("convert to ascii")
        );

        Ok(())
    }

    #[ignore]
    #[test]
    fn test_wait() -> Result<()> {
        let script = "TEST_SCRIPT".to_string();

        let mut envs: BTreeMap<String, String> = BTreeMap::new();
        envs.insert(
            script.clone(),
            "./tests/scripts/print_num_for_10_sec".to_string(),
        );

        let run = Command::new(
            "/bin/bash".to_string(),
            vec!["-c".to_string(), "$TEST_SCRIPT".to_string()],
            Some(envs),
            Some(std::env::current_dir()?),
        );

        let output = run.wait().expect("not to fail");

        assert_eq!(0, output.code().expect("to be present"));

        Ok(())
    }
}
