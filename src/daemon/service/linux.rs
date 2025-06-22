use crate::common::constants::{DISABLE, ENABLE};
use crate::common::constants::{PATH, TERRAINIUMD_LINUX_SERVICE, TERRAINIUMD_LINUX_SERVICE_PATH};
use crate::common::execute::Execute;
#[mockall_double::double]
use crate::common::execute::Executor;
use crate::common::types::command::Command;
use crate::daemon::service::Service;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const SYSTEMCTL: &str = "systemctl";
const USER: &str = "--user";
const STATUS: &str = "status";
const RELOAD: &str = "daemon-reload";
const NOW: &str = "--now";
const IS_ACTIVE: &str = "is-active";
const START: &str = "start";
const STOP: &str = "stop";

pub struct LinuxService {
    path: PathBuf,
    executor: Arc<Executor>,
}

impl Service for LinuxService {
    fn is_installed(&self) -> bool {
        self.path.exists()
    }

    fn install(&self, daemon_path: Option<PathBuf>) -> Result<()> {
        if self.is_installed() {
            if !self.is_loaded()? {
                println!("loading the service!");
                self.load().context("failed to load the service")?;
                return Ok(());
            }
            bail!("service is already installed and loaded!");
        }

        let daemon_path =
            daemon_path.unwrap_or(std::env::current_exe().context("failed to get current bin")?);

        let service = self.get(daemon_path)?;
        std::fs::write(&self.path, &service).context("failed to write service")?;

        self.start().context("failed to start service")?;

        Ok(())
    }

    fn is_loaded(&self) -> Result<bool> {
        if !self.is_installed() {
            bail!(
                "service is not installed, run terrainiumd install-service to install the service"
            );
        }

        let command = Command::new(
            SYSTEMCTL.to_string(),
            vec![
                USER.to_string(),
                STATUS.to_string(),
                TERRAINIUMD_LINUX_SERVICE.to_string(),
            ],
            None,
        );

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute process")?;

        let error = String::from_utf8_lossy(&output.stderr);
        Ok(error.is_empty())
    }

    fn load(&self) -> Result<()> {
        if self.is_loaded()? {
            println!("service is already loaded");
            return Ok(());
        }

        // reload systemd to load service
        self.reload().context("failed to reload service")
    }

    fn unload(&self) -> Result<()> {
        self.reload().context("failed to reload the service")
    }

    fn remove(&self) -> Result<()> {
        if !self.is_installed() {
            bail!("service is not installed!");
        }
        std::fs::remove_file(&self.path).context("failed to remove service file")?;
        self.unload().context("failed to unload")?;
        Ok(())
    }

    fn enable(&self, now: bool) -> Result<()> {
        if !self.is_loaded()? {
            self.load().context("failed to load the service")?;
        }

        let mut args = vec![
            USER.to_string(),
            ENABLE.to_string(),
            TERRAINIUMD_LINUX_SERVICE.to_string(),
        ];

        if now {
            args.push(NOW.to_string());
        }

        // enable service
        let command = Command::new(SYSTEMCTL.to_string(), args, None);

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute process")?;

        if !output.status.success() {
            bail!(
                "failed to enable service: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    fn disable(&self) -> Result<()> {
        if !self.is_loaded()? {
            self.load().context("failed to load the service")?;
        }

        // disable service
        let command = Command::new(
            SYSTEMCTL.to_string(),
            vec![
                USER.to_string(),
                DISABLE.to_string(),
                TERRAINIUMD_LINUX_SERVICE.to_string(),
            ],
            None,
        );

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute process")?;

        if !output.status.success() {
            bail!(
                "failed to disable service: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    fn is_running(&self) -> Result<bool> {
        self.load().context("failed to load the service")?;

        // let pid_file = Path::new(TERRAINIUMD_PID_FILE);
        // if !pid_file.exists() {
        //     return Ok(false);
        // }
        //
        // let pid =
        //     std::fs::read_to_string(pid_file).context("failed to read terrainiumd pid file")?;

        let is_running = Command::new(
            SYSTEMCTL.to_string(),
            vec![
                USER.to_string(),
                IS_ACTIVE.to_string(),
                TERRAINIUMD_LINUX_SERVICE.to_string(),
            ],
            None,
        );

        let running = self
            .executor
            .wait(None, is_running, true)
            .context("failed to check if service is running")?;

        Ok(running.success())
    }

    fn start(&self) -> Result<()> {
        if self.is_running()? {
            bail!("service is already running!");
        }

        // start service
        let command = Command::new(
            SYSTEMCTL.to_string(),
            vec![
                USER.to_string(),
                START.to_string(),
                TERRAINIUMD_LINUX_SERVICE.to_string(),
            ],
            None,
        );

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute process")?;

        if !output.status.success() {
            bail!(
                "failed to start the service: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    fn stop(&self) -> Result<()> {
        if !self.is_running()? {
            bail!("service is not running!");
        }

        // stop service
        let command = Command::new(
            SYSTEMCTL.to_string(),
            vec![
                USER.to_string(),
                STOP.to_string(),
                TERRAINIUMD_LINUX_SERVICE.to_string(),
            ],
            None,
        );

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute process")?;

        if !output.status.success() {
            bail!(
                "failed to stop the service: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    fn get(&self, daemon_path: PathBuf) -> Result<String> {
        if !daemon_path.exists() {
            bail!("{} does not exist", daemon_path.display());
        }

        let service = format!(
            r#"[Unit]
Description=terrainium daemon
After=multi-user.target

[Service]
ExecStart={} --force
Environment="PATH={}"
KillSignal=SIGTERM
StandardOutput=append:/tmp/terrainiumd.stdout.log
StandardError=append:/tmp/terrainiumd.stderr.log

[Install]
WantedBy=default.target"#,
            daemon_path.display(),
            std::env::var(PATH).context("failed to get PATH")?,
        );
        Ok(service)
    }
}

impl LinuxService {
    pub(crate) fn init(home_dir: &Path, executor: Arc<Executor>) -> Result<Box<dyn Service>> {
        let path = home_dir.join(format!(
            "{TERRAINIUMD_LINUX_SERVICE_PATH}/{TERRAINIUMD_LINUX_SERVICE}"
        ));

        if !path.parent().unwrap().exists() {
            std::fs::create_dir_all(path.parent().unwrap())
                .context("failed to create services directory")?;
        }

        Ok(Box::new(Self { path, executor }))
    }

    fn reload(&self) -> Result<()> {
        let command = Command::new(
            SYSTEMCTL.to_string(),
            vec![USER.to_string(), RELOAD.to_string()],
            None,
        );

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute process")?;

        if !output.status.success() {
            bail!(
                "failed to load the service: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::client::test_utils::assertions::executor::{AssertExecutor, ExpectedCommand};
    use crate::client::test_utils::{restore_env_var, set_env_var};
    use crate::common::constants::{
        DISABLE, ENABLE, PATH, TERRAINIUMD_LINUX_SERVICE, TERRAINIUMD_LINUX_SERVICE_PATH,
    };
    use crate::common::execute::MockExecutor;
    use crate::common::types::command::Command;
    use crate::daemon::service::linux::{
        LinuxService, IS_ACTIVE, NOW, RELOAD, START, STATUS, STOP, SYSTEMCTL, USER,
    };
    use crate::daemon::service::tests::{cleanup_test_daemon_binary, create_test_daemon_binary};
    use anyhow::Result;
    use serial_test::serial;
    use std::env::VarError;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use tempfile::tempdir;

    enum Status {
        Running,
        NotRunning,
        NotLoaded,
        NotInstalled,
    }

    fn expect_is_running(success: bool, executor: MockExecutor) -> MockExecutor {
        AssertExecutor::with(executor).wait_for(
            None,
            ExpectedCommand {
                command: Command::new(
                    SYSTEMCTL.to_string(),
                    vec![
                        USER.to_string(),
                        IS_ACTIVE.to_string(),
                        TERRAINIUMD_LINUX_SERVICE.to_string(),
                    ],
                    None,
                ),
                exit_code: if success { 0 } else { 1 },
                should_error: false,
                output: "".to_string(),
            },
            true,
            1,
        )
    }

    fn expect_status_mocks(
        home_dir: &Path,
        status: Status,
        executor: MockExecutor,
    ) -> MockExecutor {
        match status {
            Status::Running => {
                create_service_file(home_dir).unwrap();
                let executor = expect_is_loaded(true, executor, 2);
                expect_is_running(true, executor)
            }
            Status::NotRunning => {
                create_service_file(home_dir).unwrap();
                let executor = expect_is_loaded(true, executor, 2);
                expect_is_running(false, executor)
            }
            Status::NotLoaded => {
                create_service_file(home_dir).unwrap();
                expect_is_loaded(false, executor, 1)
            }
            Status::NotInstalled => executor,
        }
    }

    fn expect_is_loaded(success: bool, executor: MockExecutor, times: usize) -> MockExecutor {
        AssertExecutor::with(executor)
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(
                        SYSTEMCTL.to_string(),
                        vec![
                            USER.to_string(),
                            STATUS.to_string(),
                            TERRAINIUMD_LINUX_SERVICE.to_string(),
                        ],
                        None,
                    ),
                    exit_code: if success { 0 } else { 1 },
                    should_error: !success,
                    output: if success {
                        "".to_string()
                    } else {
                        "error".to_string()
                    },
                },
                times,
            )
            .successfully()
    }

    fn expect_load(executor: MockExecutor) -> MockExecutor {
        AssertExecutor::with(executor)
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(
                        SYSTEMCTL.to_string(),
                        vec![USER.to_string(), RELOAD.to_string()],
                        None,
                    ),
                    exit_code: 0,
                    should_error: false,
                    output: "".to_string(),
                },
                1,
            )
            .successfully()
    }

    fn expect_unload(executor: MockExecutor) -> MockExecutor {
        AssertExecutor::with(executor)
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(
                        SYSTEMCTL.to_string(),
                        vec![USER.to_string(), RELOAD.to_string()],
                        None,
                    ),
                    exit_code: 0,
                    should_error: false,
                    output: "".to_string(),
                },
                1,
            )
            .successfully()
    }

    fn expect_enable(executor: MockExecutor, now: bool) -> MockExecutor {
        let mut args = vec![
            USER.to_string(),
            ENABLE.to_string(),
            TERRAINIUMD_LINUX_SERVICE.to_string(),
        ];
        if now {
            args.push(NOW.to_string());
        }
        AssertExecutor::with(executor)
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(SYSTEMCTL.to_string(), args, None),
                    exit_code: 0,
                    should_error: false,
                    output: "".to_string(),
                },
                1,
            )
            .successfully()
    }

    fn expect_disable(executor: MockExecutor) -> MockExecutor {
        AssertExecutor::with(executor)
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(
                        SYSTEMCTL.to_string(),
                        vec![
                            USER.to_string(),
                            DISABLE.to_string(),
                            TERRAINIUMD_LINUX_SERVICE.to_string(),
                        ],
                        None,
                    ),
                    exit_code: 0,
                    should_error: false,
                    output: "".to_string(),
                },
                1,
            )
            .successfully()
    }

    fn expect_start(executor: MockExecutor) -> MockExecutor {
        AssertExecutor::with(executor)
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(
                        SYSTEMCTL.to_string(),
                        vec![
                            USER.to_string(),
                            START.to_string(),
                            TERRAINIUMD_LINUX_SERVICE.to_string(),
                        ],
                        None,
                    ),
                    exit_code: 0,
                    should_error: false,
                    output: "".to_string(),
                },
                1,
            )
            .successfully()
    }

    fn expect_stop(executor: MockExecutor) -> MockExecutor {
        AssertExecutor::with(executor)
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(
                        SYSTEMCTL.to_string(),
                        vec![
                            USER.to_string(),
                            STOP.to_string(),
                            TERRAINIUMD_LINUX_SERVICE.to_string(),
                        ],
                        None,
                    ),
                    exit_code: 0,
                    should_error: false,
                    output: "".to_string(),
                },
                1,
            )
            .successfully()
    }

    fn create_service_file(home_dir: &Path) -> Result<PathBuf> {
        let service_path = home_dir.join(format!(
            "{TERRAINIUMD_LINUX_SERVICE_PATH}/{TERRAINIUMD_LINUX_SERVICE}"
        ));
        std::fs::create_dir_all(service_path.parent().unwrap())?;
        std::fs::write(&service_path, "")?;
        Ok(service_path)
    }

    #[test]
    fn install_works() -> Result<()> {
        let home_dir = tempdir()?;

        // start the service
        let executor = expect_is_loaded(true, MockExecutor::new(), 1);
        let executor = expect_is_running(false, executor);
        let executor = expect_start(executor);

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;
        service.install(None)?;

        assert!(home_dir
            .path()
            .join(format!(
                "{TERRAINIUMD_LINUX_SERVICE_PATH}/{TERRAINIUMD_LINUX_SERVICE}"
            ))
            .exists());
        assert!(service.is_installed());
        Ok(())
    }

    #[serial]
    #[test]
    fn install_with_daemon_path() -> Result<()> {
        let path: Result<String, VarError>;
        unsafe { path = set_env_var(PATH, Some("/usr/local/bin:/usr/bin:/bin")) }
        let home_dir = tempdir()?;

        // emulate service is not loaded by returning exit code 1
        let executor = expect_is_loaded(true, MockExecutor::new(), 1);
        let executor = expect_is_running(false, executor);
        let executor = expect_start(executor);

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;

        service.install(Some(create_test_daemon_binary()?))?;
        assert!(service.is_installed());

        let contents = std::fs::read_to_string(home_dir.path().join(format!(
            "{TERRAINIUMD_LINUX_SERVICE_PATH}/{TERRAINIUMD_LINUX_SERVICE}"
        )))?;
        let expected = std::fs::read_to_string("./tests/data/terrainium.service")?;

        assert_eq!(contents, expected);

        cleanup_test_daemon_binary()?;
        unsafe { restore_env_var(PATH, path) }
        Ok(())
    }

    #[test]
    fn install_loads_if_installed_but_not_loaded() -> Result<()> {
        let home_dir = tempdir()?;

        // installed
        let service_file = create_service_file(home_dir.path())?;

        // emulate service is not loaded by returning exit code 1
        let executor = expect_is_loaded(false, MockExecutor::new(), 2);
        // load the service
        let executor = expect_load(executor);

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;
        service.install(None)?;

        assert!(service_file.exists());
        assert!(service.is_installed());
        Ok(())
    }

    #[test]
    fn install_with_daemon_path_errors_no_daemon() -> Result<()> {
        let home_dir = tempdir()?;

        let service = LinuxService::init(home_dir.path(), Arc::new(MockExecutor::new()))?;
        let error = service
            .install(Some(PathBuf::from("/non_existent")))
            .expect_err("expected error")
            .to_string();

        assert_eq!(error, "/non_existent does not exist");
        Ok(())
    }

    #[test]
    fn install_throw_an_error_already_installed() -> Result<()> {
        let home_dir = tempdir()?;

        create_service_file(home_dir.path())?;

        // emulate service is loaded by returning success
        let executor = expect_is_loaded(true, MockExecutor::new(), 1);

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;

        let error = service
            .install(None)
            .expect_err("expected error")
            .to_string();

        assert_eq!(error, "service is already installed and loaded!");

        Ok(())
    }

    #[test]
    fn remove_works() -> Result<()> {
        let home_dir = tempdir()?;

        create_service_file(home_dir.path())?;

        // emulate service is loaded by returning success
        let executor = expect_unload(MockExecutor::new());

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;

        service.remove()?;

        assert!(!service.is_installed());

        Ok(())
    }

    #[test]
    fn remove_throws_error_if_not_installed() -> Result<()> {
        let home_dir = tempdir()?;

        let service = LinuxService::init(home_dir.path(), Arc::new(MockExecutor::new()))?;

        let error = service.remove().expect_err("expected error").to_string();

        assert_eq!(error, "service is not installed!");

        Ok(())
    }

    #[test]
    fn enable_works() -> Result<()> {
        let home_dir = tempdir()?;
        create_service_file(home_dir.path())?;
        let executor = expect_is_loaded(true, MockExecutor::new(), 1);
        let executor = expect_enable(executor, false);

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;
        service.enable(false)?;

        Ok(())
    }

    #[test]
    fn enable_works_with_now() -> Result<()> {
        let home_dir = tempdir()?;

        create_service_file(home_dir.path())?;

        // setup mocks
        let executor = expect_is_loaded(true, MockExecutor::new(), 1);
        let executor = expect_enable(executor, true);

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;
        service.enable(true)?;

        Ok(())
    }

    #[test]
    fn disable_works() -> Result<()> {
        let home_dir = tempdir()?;
        create_service_file(home_dir.path())?;
        let executor = expect_is_loaded(true, MockExecutor::new(), 1);
        let executor = expect_disable(executor);

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;
        service.disable()?;

        Ok(())
    }

    #[test]
    fn start_works() -> Result<()> {
        let home_dir = tempdir()?;
        create_service_file(home_dir.path())?;

        let executor = expect_is_loaded(true, MockExecutor::new(), 1);
        let executor = expect_is_running(false, executor);
        let executor = expect_start(executor);

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;

        service.start()?;

        Ok(())
    }

    #[test]
    fn start_throws_an_error_if_already_running() -> Result<()> {
        let home_dir = tempdir()?;
        create_service_file(home_dir.path())?;

        let executor = expect_is_loaded(true, MockExecutor::new(), 1);
        let executor = expect_is_running(true, executor);

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;

        let error = service.start().expect_err("expected error").to_string();
        assert_eq!(error, "service is already running!");

        Ok(())
    }

    #[test]
    fn start_loads_service_if_not_loaded() -> Result<()> {
        let home_dir = tempdir()?;
        create_service_file(home_dir.path())?;

        let executor = expect_is_loaded(false, MockExecutor::new(), 1);
        let executor = expect_load(executor);
        let executor = expect_is_running(false, executor);
        let executor = expect_start(executor);

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;

        service.start()?;

        Ok(())
    }

    #[test]
    fn stop_works() -> Result<()> {
        let home_dir = tempdir()?;
        create_service_file(home_dir.path())?;

        let executor = expect_is_loaded(true, MockExecutor::new(), 1);
        let executor = expect_is_running(true, executor);
        let executor = expect_stop(executor);

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;

        service.stop()?;

        Ok(())
    }

    #[test]
    fn stop_throws_an_error_if_not_running() -> Result<()> {
        let home_dir = tempdir()?;
        create_service_file(home_dir.path())?;

        let executor = expect_is_loaded(true, MockExecutor::new(), 1);
        let executor = expect_is_running(false, executor);

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;

        let error = service.stop().expect_err("expected error").to_string();
        assert_eq!(error, "service is not running!");

        Ok(())
    }

    #[test]
    fn stop_loads_service_if_not_loaded() -> Result<()> {
        let home_dir = tempdir()?;
        create_service_file(home_dir.path())?;

        let executor = expect_is_loaded(false, MockExecutor::new(), 1);
        let executor = expect_load(executor);
        let executor = expect_is_running(true, executor);
        let executor = expect_stop(executor);

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;

        service.stop()?;

        Ok(())
    }

    #[test]
    fn status_not_installed() -> Result<()> {
        let home_dir = tempdir()?;
        let executor =
            expect_status_mocks(home_dir.path(), Status::NotInstalled, MockExecutor::new());

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;

        assert_eq!("not installed", service.status()?);
        Ok(())
    }

    #[test]
    fn status_not_loaded() -> Result<()> {
        let home_dir = tempdir()?;
        let executor = expect_status_mocks(home_dir.path(), Status::NotLoaded, MockExecutor::new());

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;

        assert_eq!("not loaded", service.status()?);
        Ok(())
    }

    #[test]
    fn status_not_running() -> Result<()> {
        let home_dir = tempdir()?;
        let executor =
            expect_status_mocks(home_dir.path(), Status::NotRunning, MockExecutor::new());

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;

        assert_eq!("not running", service.status()?);
        Ok(())
    }

    #[test]
    fn status_running() -> Result<()> {
        let home_dir = tempdir()?;
        let executor = expect_status_mocks(home_dir.path(), Status::Running, MockExecutor::new());

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;

        assert_eq!("running", service.status()?);
        Ok(())
    }
}
