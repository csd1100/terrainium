use std::collections::BTreeMap;

use anyhow::Result;

#[cfg(test)]
use mockall::automock;
use mockall_double::double;

use crate::{
    helpers::{
        constants::{TERRAINIUM_DEV, TERRAINIUM_EXECUTOR, TERRAINIUM_EXECUTOR_ENV},
        operations::get_process_log_file,
    },
    types::commands::Command,
};

#[double]
use crate::types::executor::Executable;

#[double]
use crate::shell::process::spawn;

#[cfg_attr(test, automock)]
pub mod processes {
    use std::collections::BTreeMap;

    use anyhow::{anyhow, Result};

    use crate::{helpers::constants::TERRAINIUM_SESSION_ID, types::commands::Command};

    pub fn start(background: Vec<Command>, envs: BTreeMap<String, String>) -> Result<()> {
        if let Some(session_id) = envs.get(TERRAINIUM_SESSION_ID) {
            super::iterate_over_commands_and_spawn(session_id, background, envs.clone())?;
        } else if let Ok(session_id) = std::env::var(TERRAINIUM_SESSION_ID) {
            super::iterate_over_commands_and_spawn(&session_id, background, envs.clone())?;
        } else {
            return Err(anyhow!(
                "unable to get terrainium session id to start background processes"
            ));
        }
        Ok(())
    }
}

fn start_process_with_session_id(
    session_id: String,
    command: Command,
    envs: Option<BTreeMap<String, String>>,
) -> Result<()> {
    let exec_arg_json: Executable = command.into();
    let exec_arg = serde_json::to_string(&exec_arg_json)?;
    let mut command = TERRAINIUM_EXECUTOR.to_string();

    let dev = std::env::var(TERRAINIUM_DEV);
    if dev.is_ok() && dev? == *"true" {
        if let Ok(executor) = std::env::var(TERRAINIUM_EXECUTOR_ENV) {
            command = executor;
        }
    }

    let args = vec!["--id", &session_id, "--exec", &exec_arg];

    let (_, spawn_out) = get_process_log_file(
        &session_id,
        format!("spawn-out-{}.log", exec_arg_json.get_uuid()),
    )?;
    let (_, spawn_err) = get_process_log_file(
        &session_id,
        format!("spawn-err-{}.log", exec_arg_json.get_uuid()),
    )?;

    spawn::with_stdout_stderr(&command, args, envs, Some(spawn_out), Some(spawn_err))?;

    Ok(())
}

fn iterate_over_commands_and_spawn(
    session_id: &String,
    background: Vec<Command>,
    envs: BTreeMap<String, String>,
) -> Result<()> {
    let errors: Result<Vec<_>> = background
        .into_iter()
        .map(|command| {
            start_process_with_session_id(session_id.to_string(), command, Some(envs.clone()))
        })
        .collect();

    if let Some(e) = errors.err() {
        Err(e)
    } else {
        Ok(())
    }
}

// #[cfg(test)]
// mod test {
//     use std::{collections::BTreeMap, fs::File, path::PathBuf};
//
//     use anyhow::Result;
//     use mockall::predicate::eq;
//     use serial_test::serial;
//
//     use crate::{
//         helpers::{
//             constants::{TERRAINIUM_DEV, TERRAINIUM_EXECUTOR_ENV},
//             operations::mock_fs,
//         },
//         shell::process::mock_spawn,
//         types::{
//             commands::Command,
//             executor::{Executable, MockExecutable},
//         },
//     };
//
//     #[test]
//     #[serial]
//     fn spawns_all_background_processes_if_env_passed() -> Result<()> {
//         File::create("/tmp/test")?;
//
//         let mock_executable_from_command = MockExecutable::from_context();
//         mock_executable_from_command
//             .expect()
//             .with(eq(Command {
//                 exe: "command1".to_string(),
//                 args: None,
//             }))
//             .returning(|comm| {
//                 let mut mock = MockExecutable::new();
//                 mock.expect_get_uuid().return_const("id-1");
//                 mock.expect_private_serialize().return_once(|| Executable {
//                     uuid: "id-1".to_string(),
//                     exe: comm.exe,
//                     args: comm.args,
//                 });
//                 mock
//             });
//
//         mock_executable_from_command
//             .expect()
//             .with(eq(Command {
//                 exe: "command2".to_string(),
//                 args: Some(vec!["args2".to_string()]),
//             }))
//             .returning(|comm| {
//                 let mut mock = MockExecutable::new();
//                 mock.expect_get_uuid().return_const("id-2");
//                 mock.expect_private_serialize().return_once(|| Executable {
//                     uuid: "id-2".to_string(),
//                     exe: comm.exe,
//                     args: comm.args,
//                 });
//                 mock
//             });
//
//         let mock_log_file = mock_fs::get_process_log_file_context();
//         mock_log_file
//             .expect()
//             .with(
//                 eq("session_id".to_string()),
//                 eq("spawn-out-id-1.log".to_string()),
//             )
//             .return_once(|_, _| Ok((PathBuf::from("/tmp/test.log"), get_test_file()?)));
//         mock_log_file
//             .expect()
//             .with(
//                 eq("session_id".to_string()),
//                 eq("spawn-err-id-1.log".to_string()),
//             )
//             .return_once(|_, _| Ok((PathBuf::from("/tmp/test.log"), get_test_file()?)));
//
//         mock_log_file
//             .expect()
//             .with(
//                 eq("session_id".to_string()),
//                 eq("spawn-out-id-2.log".to_string()),
//             )
//             .return_once(|_, _| Ok((PathBuf::from("/tmp/test.log"), get_test_file()?)));
//         mock_log_file
//             .expect()
//             .with(
//                 eq("session_id".to_string()),
//                 eq("spawn-err-id-2.log".to_string()),
//             )
//             .return_once(|_, _| Ok((PathBuf::from("/tmp/test.log"), get_test_file()?)));
//
//         let mock_spawn_process = mock_spawn::with_stdout_stderr_context();
//         mock_spawn_process
//             .expect()
//             .withf(|exe, args, envs, _, _| {
//                 let mut command = "terrainium_executor".to_string();
//                 let dev = std::env::var(TERRAINIUM_DEV);
//                 if dev.is_ok() && dev.unwrap() == *"true" {
//                     if let Ok(executor) = std::env::var(TERRAINIUM_EXECUTOR_ENV) {
//                         command = executor;
//                     }
//                 }
//                 let exe_eq = exe == command;
//
//                 let exec_json = serde_json::to_string(&Executable {
//                     uuid: "id-1".to_string(),
//                     exe: "command1".to_string(),
//                     args: None,
//                 })
//                 .expect("to be parsed");
//                 let args_eq = *args == vec!["--id", "session_id", "--exec", &exec_json];
//
//                 let mut expected = BTreeMap::<String, String>::new();
//                 expected.insert(
//                     "TERRAINIUM_SESSION_ID".to_string(),
//                     "session_id".to_string(),
//                 );
//                 expected.insert("TEST".to_string(), "value".to_string());
//                 let envs_eq = *envs.as_ref().unwrap() == expected;
//
//                 exe_eq && args_eq && envs_eq
//             })
//             .return_once(|_, _, _, _, _| Ok(()));
//
//         mock_spawn_process
//             .expect()
//             .withf(|exe, args, envs, _, _| {
//                 let mut command = "terrainium_executor".to_string();
//                 let dev = std::env::var(TERRAINIUM_DEV);
//                 if dev.is_ok() && dev.unwrap() == *"true" {
//                     if let Ok(executor) = std::env::var(TERRAINIUM_EXECUTOR_ENV) {
//                         command = executor;
//                     }
//                 }
//                 let exe_eq = exe == command;
//
//                 let exec_json = serde_json::to_string(&Executable {
//                     uuid: "id-2".to_string(),
//                     exe: "command2".to_string(),
//                     args: Some(vec!["args2".to_string()]),
//                 })
//                 .expect("to be parsed");
//                 let args_eq = *args == vec!["--id", "session_id", "--exec", &exec_json];
//
//                 let mut expected = BTreeMap::<String, String>::new();
//                 expected.insert(
//                     "TERRAINIUM_SESSION_ID".to_string(),
//                     "session_id".to_string(),
//                 );
//                 expected.insert("TEST".to_string(), "value".to_string());
//                 let envs_eq = *envs.as_ref().unwrap() == expected;
//
//                 exe_eq && args_eq && envs_eq
//             })
//             .return_once(|_, _, _, _, _| Ok(()));
//
//         let commands = vec![
//             Command {
//                 exe: "command1".to_string(),
//                 args: None,
//             },
//             Command {
//                 exe: "command2".to_string(),
//                 args: Some(vec!["args2".to_string()]),
//             },
//         ];
//
//         let mut envs = BTreeMap::<String, String>::new();
//         envs.insert(
//             "TERRAINIUM_SESSION_ID".to_string(),
//             "session_id".to_string(),
//         );
//         envs.insert("TEST".to_string(), "value".to_string());
//
//         super::processes::start(commands, envs)?;
//
//         Ok(())
//     }
//
//     #[test]
//     #[serial]
//     fn spawns_all_background_processes_if_env_var_set() -> Result<()> {
//         // setup
//         let real_session_id = std::env::var("TERRAINIUM_SESSION_ID").ok();
//         std::env::set_var("TERRAINIUM_SESSION_ID", "session_id");
//         File::create("/tmp/test")?;
//
//         let mock_executable_from_command = MockExecutable::from_context();
//         mock_executable_from_command
//             .expect()
//             .with(eq(Command {
//                 exe: "command1".to_string(),
//                 args: None,
//             }))
//             .returning(|comm| {
//                 let mut mock = MockExecutable::new();
//                 mock.expect_get_uuid().return_const("id-1");
//                 mock.expect_private_serialize().return_once(|| Executable {
//                     uuid: "id-1".to_string(),
//                     exe: comm.exe,
//                     args: comm.args,
//                 });
//                 mock
//             });
//
//         mock_executable_from_command
//             .expect()
//             .with(eq(Command {
//                 exe: "command2".to_string(),
//                 args: Some(vec!["args2".to_string()]),
//             }))
//             .returning(|comm| {
//                 let mut mock = MockExecutable::new();
//                 mock.expect_get_uuid().return_const("id-2");
//                 mock.expect_private_serialize().return_once(|| Executable {
//                     uuid: "id-2".to_string(),
//                     exe: comm.exe,
//                     args: comm.args,
//                 });
//                 mock
//             });
//
//         let mock_log_file = mock_fs::get_process_log_file_context();
//         mock_log_file
//             .expect()
//             .with(
//                 eq("session_id".to_string()),
//                 eq("spawn-out-id-1.log".to_string()),
//             )
//             .return_once(|_, _| Ok((PathBuf::from("/tmp/test.log"), get_test_file()?)));
//         mock_log_file
//             .expect()
//             .with(
//                 eq("session_id".to_string()),
//                 eq("spawn-err-id-1.log".to_string()),
//             )
//             .return_once(|_, _| Ok((PathBuf::from("/tmp/test.log"), get_test_file()?)));
//
//         mock_log_file
//             .expect()
//             .with(
//                 eq("session_id".to_string()),
//                 eq("spawn-out-id-2.log".to_string()),
//             )
//             .return_once(|_, _| Ok((PathBuf::from("/tmp/test.log"), get_test_file()?)));
//         mock_log_file
//             .expect()
//             .with(
//                 eq("session_id".to_string()),
//                 eq("spawn-err-id-2.log".to_string()),
//             )
//             .return_once(|_, _| Ok((PathBuf::from("/tmp/test.log"), get_test_file()?)));
//
//         let mock_spawn_process = mock_spawn::with_stdout_stderr_context();
//         mock_spawn_process
//             .expect()
//             .withf(|exe, args, envs, _, _| {
//                 let mut command = "terrainium_executor".to_string();
//                 let dev = std::env::var(TERRAINIUM_DEV);
//                 if dev.is_ok() && dev.unwrap() == *"true" {
//                     if let Ok(executor) = std::env::var(TERRAINIUM_EXECUTOR_ENV) {
//                         command = executor;
//                     }
//                 }
//                 let exe_eq = exe == command;
//
//                 let exec_json = serde_json::to_string(&Executable {
//                     uuid: "id-1".to_string(),
//                     exe: "command1".to_string(),
//                     args: None,
//                 })
//                 .expect("to be parsed");
//                 let args_eq = *args == vec!["--id", "session_id", "--exec", &exec_json];
//
//                 let mut expected = BTreeMap::<String, String>::new();
//                 expected.insert("TEST".to_string(), "value".to_string());
//                 let envs_eq = *envs.as_ref().unwrap() == expected;
//
//                 exe_eq && args_eq && envs_eq
//             })
//             .return_once(|_, _, _, _, _| Ok(()));
//
//         mock_spawn_process
//             .expect()
//             .withf(|exe, args, envs, _, _| {
//                 let mut command = "terrainium_executor".to_string();
//                 let dev = std::env::var(TERRAINIUM_DEV);
//                 if dev.is_ok() && dev.unwrap() == *"true" {
//                     if let Ok(executor) = std::env::var(TERRAINIUM_EXECUTOR_ENV) {
//                         command = executor;
//                     }
//                 }
//                 let exe_eq = exe == command;
//
//                 let exec_json = serde_json::to_string(&Executable {
//                     uuid: "id-2".to_string(),
//                     exe: "command2".to_string(),
//                     args: Some(vec!["args2".to_string()]),
//                 })
//                 .expect("to be parsed");
//                 let args_eq = *args == vec!["--id", "session_id", "--exec", &exec_json];
//
//                 let mut expected = BTreeMap::<String, String>::new();
//                 expected.insert("TEST".to_string(), "value".to_string());
//                 let envs_eq = *envs.as_ref().unwrap() == expected;
//
//                 exe_eq && args_eq && envs_eq
//             })
//             .return_once(|_, _, _, _, _| Ok(()));
//
//         let commands = vec![
//             Command {
//                 exe: "command1".to_string(),
//                 args: None,
//             },
//             Command {
//                 exe: "command2".to_string(),
//                 args: Some(vec!["args2".to_string()]),
//             },
//         ];
//
//         let mut envs = BTreeMap::<String, String>::new();
//         envs.insert("TEST".to_string(), "value".to_string());
//
//         super::processes::start(commands, envs)?;
//         // cleanup
//         if let Some(session_id) = real_session_id {
//             std::env::set_var("TERRAINIUM_SESSION_ID", session_id)
//         } else {
//             std::env::remove_var("TERRAINIUM_SESSION_ID")
//         }
//
//         Ok(())
//     }
//
//     #[test]
//     #[serial]
//     fn returns_err_if_no_session_id() -> Result<()> {
//         let real_session_id = std::env::var("TERRAINIUM_SESSION_ID").ok();
//         std::env::remove_var("TERRAINIUM_SESSION_ID");
//
//         let commands = vec![
//             Command {
//                 exe: "command1".to_string(),
//                 args: None,
//             },
//             Command {
//                 exe: "command2".to_string(),
//                 args: Some(vec!["args2".to_string()]),
//             },
//         ];
//
//         let mut envs = BTreeMap::<String, String>::new();
//         envs.insert("TEST".to_string(), "value".to_string());
//         let actual = super::processes::start(commands, envs)
//             .unwrap_err()
//             .to_string();
//
//         assert_eq!(
//             "unable to get terrainium session id to start background processes".to_string(),
//             actual
//         );
//
//         // cleanup
//         if let Some(session_id) = real_session_id {
//             std::env::set_var("TERRAINIUM_SESSION_ID", session_id)
//         } else {
//             std::env::remove_var("TERRAINIUM_SESSION_ID")
//         }
//         Ok(())
//     }
//
//     fn get_test_file() -> Result<File> {
//         Ok(File::open("/tmp/test")?)
//     }
// }
