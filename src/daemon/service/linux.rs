use crate::common::constants::{DISABLE, ENABLE};
use crate::common::constants::{PATH, TERRAINIUMD_LINUX_SERVICE, TERRAINIUMD_LINUX_SERVICE_PATH};
use crate::common::execute::Execute;
#[mockall_double::double]
use crate::common::execute::Executor;
use crate::common::types::command::Command;
use crate::daemon::service::{
    Service, ERROR_ALREADY_RUNNING, ERROR_IS_NOT_RUNNING, ERROR_SERVICE_NOT_INSTALLED,
};
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

    fn install(&self) -> Result<()> {
        if self.is_installed() {
            self.load()?;
            return Ok(());
        }

        let service = self.get(true)?;
        std::fs::write(&self.path, &service).context("failed to write service")?;

        self.load()?;
        self.start()?;

        Ok(())
    }

    fn is_loaded(&self) -> Result<bool> {
        if !self.is_installed() {
            bail!(ERROR_SERVICE_NOT_INSTALLED,);
        }

        let command = Command::new(
            SYSTEMCTL.to_string(),
            vec![
                USER.to_string(),
                STATUS.to_string(),
                TERRAINIUMD_LINUX_SERVICE.to_string(),
            ],
            Some(std::env::temp_dir()),
        );

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute status command")?;

        let error = String::from_utf8_lossy(&output.stderr);
        Ok(error.is_empty())
    }

    fn load(&self) -> Result<()> {
        if self.is_loaded()? {
            println!("service is already loaded");
            return Ok(());
        }

        // reload systemd to load service
        self.reload().context("failed to reload the services")
    }

    fn unload(&self) -> Result<()> {
        if !self.is_loaded()? {
            println!("service is already unloaded");
            return Ok(());
        }
        self.reload().context("failed to reload the services")
    }

    fn remove(&self) -> Result<()> {
        if !self.is_installed() {
            bail!(ERROR_SERVICE_NOT_INSTALLED);
        }
        std::fs::remove_file(&self.path).context("failed to remove service file")?;
        self.reload().context("failed to reload the services")
    }

    fn enable(&self, now: bool) -> Result<()> {
        self.load()?;

        let mut args = vec![
            USER.to_string(),
            ENABLE.to_string(),
            TERRAINIUMD_LINUX_SERVICE.to_string(),
        ];

        if now {
            args.push(NOW.to_string());
        }

        // enable service
        let command = Command::new(SYSTEMCTL.to_string(), args, Some(std::env::temp_dir()));

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

    fn disable(&self, now: bool) -> Result<()> {
        self.load()?;

        let mut args = vec![
            USER.to_string(),
            DISABLE.to_string(),
            TERRAINIUMD_LINUX_SERVICE.to_string(),
        ];

        if now {
            args.push(NOW.to_string());
        }

        // disable service
        let command = Command::new(SYSTEMCTL.to_string(), args, Some(std::env::temp_dir()));

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
        let is_running = Command::new(
            SYSTEMCTL.to_string(),
            vec![
                USER.to_string(),
                IS_ACTIVE.to_string(),
                TERRAINIUMD_LINUX_SERVICE.to_string(),
            ],
            Some(std::env::temp_dir()),
        );

        let running = self
            .executor
            .wait(None, is_running, true)
            .context("failed to check if service is running")?;

        Ok(running.success())
    }

    fn start(&self) -> Result<()> {
        if self.is_running()? {
            bail!(ERROR_ALREADY_RUNNING);
        }

        // start service
        let command = Command::new(
            SYSTEMCTL.to_string(),
            vec![
                USER.to_string(),
                START.to_string(),
                TERRAINIUMD_LINUX_SERVICE.to_string(),
            ],
            Some(std::env::temp_dir()),
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
            bail!(ERROR_IS_NOT_RUNNING);
        }

        // stop service
        let command = Command::new(
            SYSTEMCTL.to_string(),
            vec![
                USER.to_string(),
                STOP.to_string(),
                TERRAINIUMD_LINUX_SERVICE.to_string(),
            ],
            Some(std::env::temp_dir()),
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

    #[allow(unused_variables)]
    fn get(&self, enabled: bool) -> Result<String> {
        let daemon_path = std::env::current_exe().context("failed to get current bin")?;
        if !daemon_path.exists() {
            bail!("{} does not exist", daemon_path.display());
        }

        let service = format!(
            r#"[Unit]
Description=terrainium daemon
After=multi-user.target

[Service]
ExecStart={} --run
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
            Some(std::env::temp_dir()),
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
    use crate::common::constants::{
        DISABLE, ENABLE, TERRAINIUMD_LINUX_SERVICE, TERRAINIUMD_LINUX_SERVICE_PATH,
    };
    use crate::common::execute::MockExecutor;
    use crate::common::types::command::Command;
    use crate::daemon::service::linux::{
        LinuxService, IS_ACTIVE, NOW, RELOAD, START, STATUS, STOP, SYSTEMCTL, USER,
    };
    use crate::daemon::service::tests::Status;
    use crate::daemon::service::ERROR_SERVICE_NOT_INSTALLED;
    use anyhow::Result;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use tempfile::tempdir;

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
                    Some(std::env::temp_dir()),
                ),
                exit_code: if success { 0 } else { 1 },
                should_fail_to_execute: false,
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
                let executor = expect_is_loaded(true, executor);
                expect_is_running(true, executor)
            }
            Status::NotRunning => {
                create_service_file(home_dir).unwrap();
                let executor = expect_is_loaded(true, executor);
                expect_is_running(false, executor)
            }
            Status::NotLoaded => {
                create_service_file(home_dir).unwrap();
                expect_is_loaded(false, executor)
            }
            Status::NotInstalled => executor,
        }
    }

    fn expect_is_loaded(success: bool, executor: MockExecutor) -> MockExecutor {
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
                        Some(std::env::temp_dir()),
                    ),
                    exit_code: if success { 0 } else { 1 },
                    should_fail_to_execute: false,
                    output: if success {
                        "".to_string()
                    } else {
                        "error".to_string()
                    },
                },
                1,
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
                        Some(std::env::temp_dir()),
                    ),
                    exit_code: 0,
                    should_fail_to_execute: false,
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
                        Some(std::env::temp_dir()),
                    ),
                    exit_code: 0,
                    should_fail_to_execute: false,
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
                    command: Command::new(SYSTEMCTL.to_string(), args, Some(std::env::temp_dir())),
                    exit_code: 0,
                    should_fail_to_execute: false,
                    output: "".to_string(),
                },
                1,
            )
            .successfully()
    }

    fn expect_disable(executor: MockExecutor, now: bool) -> MockExecutor {
        let mut args = vec![
            USER.to_string(),
            DISABLE.to_string(),
            TERRAINIUMD_LINUX_SERVICE.to_string(),
        ];
        if now {
            args.push(NOW.to_string());
        }
        AssertExecutor::with(executor)
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(SYSTEMCTL.to_string(), args, Some(std::env::temp_dir())),
                    exit_code: 0,
                    should_fail_to_execute: false,
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
                        Some(std::env::temp_dir()),
                    ),
                    exit_code: 0,
                    should_fail_to_execute: false,
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
                        Some(std::env::temp_dir()),
                    ),
                    exit_code: 0,
                    should_fail_to_execute: false,
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
        let executor = expect_is_loaded(false, MockExecutor::new());
        let executor = expect_load(executor);
        let executor = expect_is_running(false, executor);
        let executor = expect_start(executor);

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;
        service.install()?;

        assert!(home_dir
            .path()
            .join(format!(
                "{TERRAINIUMD_LINUX_SERVICE_PATH}/{TERRAINIUMD_LINUX_SERVICE}"
            ))
            .exists());
        assert!(service.is_installed());
        Ok(())
    }

    #[test]
    fn install_loads_if_installed_but_not_loaded() -> Result<()> {
        let home_dir = tempdir()?;

        // installed
        let service_file = create_service_file(home_dir.path())?;

        // emulate service is not loaded by returning exit code 1
        let executor = expect_is_loaded(false, MockExecutor::new());
        // load the service
        let executor = expect_load(executor);

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;
        service.install()?;

        assert!(service_file.exists());
        assert!(service.is_installed());
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

        assert_eq!(error, ERROR_SERVICE_NOT_INSTALLED);

        Ok(())
    }

    #[test]
    fn enable_works() -> Result<()> {
        let home_dir = tempdir()?;
        create_service_file(home_dir.path())?;
        let executor = expect_is_loaded(true, MockExecutor::new());
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
        let executor = expect_is_loaded(true, MockExecutor::new());
        let executor = expect_enable(executor, true);

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;
        service.enable(true)?;

        Ok(())
    }

    #[test]
    fn enable_throw_error_if_not_installed() -> Result<()> {
        let home_dir = tempdir()?;

        let service = LinuxService::init(home_dir.path(), Arc::new(MockExecutor::new()))?;

        let error = service
            .enable(false)
            .expect_err("expected error")
            .to_string();

        assert_eq!(error, ERROR_SERVICE_NOT_INSTALLED);

        Ok(())
    }

    #[test]
    fn disable_works() -> Result<()> {
        let home_dir = tempdir()?;
        create_service_file(home_dir.path())?;
        let executor = expect_is_loaded(true, MockExecutor::new());
        let executor = expect_disable(executor, false);

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;
        service.disable(false)?;

        Ok(())
    }

    #[test]
    fn disable_works_with_now() -> Result<()> {
        let home_dir = tempdir()?;

        create_service_file(home_dir.path())?;

        // setup mocks
        let executor = expect_is_loaded(true, MockExecutor::new());
        let executor = expect_disable(executor, true);

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;
        service.disable(true)?;

        Ok(())
    }

    #[test]
    fn disable_throw_error_if_not_installed() -> Result<()> {
        let home_dir = tempdir()?;

        let service = LinuxService::init(home_dir.path(), Arc::new(MockExecutor::new()))?;

        let error = service
            .disable(false)
            .expect_err("expected error")
            .to_string();

        assert_eq!(error, ERROR_SERVICE_NOT_INSTALLED);

        Ok(())
    }

    #[test]
    fn start_works() -> Result<()> {
        let home_dir = tempdir()?;
        create_service_file(home_dir.path())?;

        let executor = expect_is_running(false, MockExecutor::new());
        let executor = expect_start(executor);

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;

        service.start()?;

        Ok(())
    }

    #[test]
    fn start_throws_an_error_if_already_running() -> Result<()> {
        let home_dir = tempdir()?;
        create_service_file(home_dir.path())?;

        let executor = expect_is_running(true, MockExecutor::new());

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;

        let error = service.start().expect_err("expected error").to_string();
        assert_eq!(error, "service is already running!");

        Ok(())
    }

    #[test]
    fn stop_works() -> Result<()> {
        let home_dir = tempdir()?;
        create_service_file(home_dir.path())?;

        let executor = expect_is_running(true, MockExecutor::new());
        let executor = expect_stop(executor);

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;

        service.stop()?;

        Ok(())
    }

    #[test]
    fn stop_throws_an_error_if_not_running() -> Result<()> {
        let home_dir = tempdir()?;
        create_service_file(home_dir.path())?;

        let executor = expect_is_running(false, MockExecutor::new());

        let service = LinuxService::init(home_dir.path(), Arc::new(executor))?;

        let error = service.stop().expect_err("expected error").to_string();
        assert_eq!(error, "service is not running!");

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
