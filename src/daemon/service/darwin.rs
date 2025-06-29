use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, bail};

use crate::common::constants::{
    ENABLE, PATH, TERRAINIUMD, TERRAINIUMD_DARWIN_SERVICE_PATH, TERRAINIUMD_DEBUG,
};
use crate::common::execute::Execute;
#[mockall_double::double]
use crate::common::execute::Executor;
use crate::common::types::command::Command;
use crate::daemon::service::{
    ERROR_ALREADY_RUNNING, ERROR_IS_NOT_RUNNING, ERROR_SERVICE_NOT_INSTALLED,
    ERROR_SERVICE_NOT_LOADED, Service,
};

const GUI: &str = "gui";
const LAUNCHCTL: &str = "launchctl";
const LOAD: &str = "bootstrap";
const UNLOAD: &str = "bootout";
const PRINT: &str = "print";
const START: &str = "kickstart";
const STOP: &str = "kill";
const SIGTERM: &str = "SIGTERM";
const RUNNING: &str = "state = running";

/// Fetches current users id required for `launchctl` commands using
/// `id -u` command.
fn get_uid(executor: Arc<Executor>) -> Result<String> {
    let command = Command::new(
        "id".to_string(),
        vec!["-u".to_string()],
        Some(std::env::temp_dir()),
    );
    let output = executor.get_output(None, command)?;
    if !output.status.success() {
        bail!(
            "command to get uid exited with error: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let uid = String::from_utf8(output.stdout).context("failed to parse output")?;
    Ok(uid.replace('\n', ""))
}

/// Manage macOS `launchd` service using `launchctl` commands.
pub struct DarwinService {
    uid: String,
    path: PathBuf,
    executor: Arc<Executor>,
}

impl Service for DarwinService {
    /// Check if service `plist` file is present at:
    /// `~/Library/LaunchAgents/com.csd1100.terrainiumd.plist`
    fn is_installed(&self) -> bool {
        self.path.exists()
    }

    /// Copy com.csd1100.terrainiumd.plist file to `~/Library/LaunchAgents/com.csd1100.terrainiumd.plist`
    /// and bootstrap the service.
    fn install(&self) -> Result<()> {
        if self.is_installed() {
            self.load()?;
            return Ok(());
        }

        self.write_service(true)
            .context("failed to write service")?;

        self.enable(true)?;

        Ok(())
    }

    /// Check if service is bootstrapped by using `launchctl print gui/<uid>/com.csd1100.terrainium`
    /// command.
    fn is_loaded(&self) -> Result<bool> {
        if !self.is_installed() {
            bail!(ERROR_SERVICE_NOT_INSTALLED);
        }

        let status = self.get_status();

        Ok(status.is_ok())
    }

    /// Bootstrap using `launchctl` command.
    ///
    /// Even if service is installed the bootstrap command might fail if service
    /// is disabled by the user. User must enable the service to bootstrap and run
    /// the service
    ///
    /// `launchctl bootstrap gui/<uid> ~/Library/LaunchAgents/com.csd1100.terrainiumd.plist`
    fn load(&self) -> Result<()> {
        if self.is_loaded()? {
            return Ok(());
        }

        // bootstrap service
        let command = Command::new(
            LAUNCHCTL.to_string(),
            vec![
                LOAD.to_string(),
                self.get_target()?,
                self.path.to_str().unwrap().to_string(),
            ],
            Some(std::env::temp_dir()),
        );

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute bootstrap command")?;

        if !output.status.success() {
            bail!(
                "failed to load the service, error: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    /// Unloads the service.
    ///
    /// `launchctl bootout gui/<uid>/com.csd1100.terrainiumd`
    fn unload(&self) -> Result<()> {
        if !self.is_loaded()? {
            return Ok(());
        }

        // bootout service
        let command = Command::new(
            LAUNCHCTL.to_string(),
            vec![
                UNLOAD.to_string(),
                self.get_service_target()
                    .context("failed to get service target")?,
            ],
            Some(std::env::temp_dir()),
        );

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute bootout command")?;

        if !output.status.success() {
            bail!(
                "failed to unload the service: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    fn reload(&self) -> Result<()> {
        if !self.is_installed() {
            bail!(ERROR_SERVICE_NOT_INSTALLED)
        }

        self.unload()?;
        self.load()
    }

    /// Removes the service from `~/Library/LaunchAgents/com.csd1100.terrainiumd.plist`.
    /// Unload the service if it is loaded.
    fn remove(&self) -> Result<()> {
        self.unload()?;
        std::fs::remove_file(&self.path).context("failed to remove service file")
    }

    /// Check if service is enabled by reading the service file
    /// and checking if `RunAtLoad` is true
    fn is_enabled(&self) -> Result<bool> {
        let service = std::fs::read_to_string(&self.path).context("failed to read the service")?;
        Ok(service.contains(
            r#"<key>RunAtLoad</key>
        <true/>"#,
        ))
    }

    /// Enables the service to be bootstrapped and start at the login.
    /// The service must be enabled in order to be bootstrapped.
    ///
    /// Sets `RunAtLoad` to true for service and, runs
    /// `launchctl enable gui/<uid>/com.csd1100.terrainiumd`
    ///
    /// If `now` is true then service is enabled and started at the same time.
    fn enable(&self, now: bool) -> Result<()> {
        if !self.is_installed() {
            bail!(ERROR_SERVICE_NOT_INSTALLED);
        }

        // enable the service i.e. set `RunAtLoad` to true
        self.write_service(true)
            .context("failed to enable service")?;

        let command = Command::new(
            LAUNCHCTL.to_string(),
            vec![ENABLE.to_string(), self.get_service_target()?],
            Some(std::env::temp_dir()),
        );

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute enable command")?;

        if !output.status.success() {
            bail!(
                "failed to enable service using launchctl: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        if now {
            // unload to refresh service definition
            self.unload()?;
            // loading will auto-start service due to `RunAtLoad`
            self.load()?;
        } else {
            // load once again
            self.load()?;
        }

        Ok(())
    }

    /// Sets `RunAtLoad` to false which disables service to start when loaded.
    ///
    /// if `now` is true then service is stopped as well.
    fn disable(&self, now: bool) -> Result<()> {
        self.load()?;

        // disable the service i.e. set `RunAtLoad` to false
        self.write_service(false)
            .context("failed to disable service")?;

        if now {
            self.stop()?;
        }

        Ok(())
    }

    /// Check if terrainiumd process is running by checking if status has
    /// `state = running` in the output.
    /// The status is checked by `launchctl print gui/<uid>/com.csd1100.terrainiumd`
    /// command.
    ///
    /// `should_check_loaded` defines to check if service is loaded, status
    /// command has already checked if service is loaded or not so only in
    /// that case it should be false. In case of start and stop it should
    /// be true.
    fn is_running(&self, should_check_loaded: bool) -> Result<bool> {
        if should_check_loaded && !self.is_installed() {
            bail!(ERROR_SERVICE_NOT_INSTALLED)
        }

        let result = self.get_status();
        match result {
            Ok(status) => Ok(status.contains(RUNNING)),
            Err(_) => {
                bail!(ERROR_SERVICE_NOT_LOADED)
            }
        }
    }

    /// Start the service if it is not already running.
    ///
    /// `launchctl kickstart gui/<uid>/com.csd1100.terrainiumd`
    fn start(&self) -> Result<()> {
        if self.is_running(true)? {
            bail!(ERROR_ALREADY_RUNNING);
        }

        // start service
        let command = Command::new(
            LAUNCHCTL.to_string(),
            vec![START.to_string(), self.get_service_target()?],
            Some(std::env::temp_dir()),
        );

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute kickstart command")?;

        if !output.status.success() {
            bail!(
                "failed to start the service: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Stops the service process by sending it `SIGTERM`.
    ///
    /// `launchctl kill SIGTERM gui/<uid>/com.csd1100.terrainiumd`
    fn stop(&self) -> Result<()> {
        if !self.is_running(true)? {
            bail!(ERROR_IS_NOT_RUNNING)
        }

        // stop service
        let command = Command::new(
            LAUNCHCTL.to_string(),
            vec![
                STOP.to_string(),
                SIGTERM.to_string(),
                self.get_service_target()?,
            ],
            Some(std::env::temp_dir()),
        );

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute kill process")?;

        if !output.status.success() {
            bail!(
                "failed to stop the service: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Returns service `plist` file contents.
    ///
    /// if `enabled` is true, then `RunAtLoad` is set to true in service defintion
    /// which runs the service whenever it is bootstrapped.
    fn get(&self, enabled: bool) -> Result<String> {
        let daemon_path = std::env::current_exe().context("failed to get current bin")?;

        if !daemon_path.exists() {
            bail!("{} does not exist", daemon_path.display());
        }

        let service = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
    <dict>
        <key>Label</key>
        <string>{}</string>
        <key>ProgramArguments</key>
        <array>
            <string>{}</string>
            <string>--run</string>
        </array>
        <key>EnvironmentVariables</key>
        <dict>
            <key>PATH</key>
            <string>{}</string>
        </dict>
        <key>RunAtLoad</key>
        <{enabled}/>
        <key>StandardOutPath</key>
        <string>/tmp/{3}.stdout.log</string>
        <key>StandardErrorPath</key>
        <string>/tmp/{3}.stderr.log</string>
        <key>ProcessType</key>
        <string>Background</string>
    </dict>
</plist>"#,
            Self::get_name(),
            daemon_path.display(),
            std::env::var(PATH).context("failed to get PATH")?,
            Self::get_service_identifier()
        );
        Ok(service)
    }
}

impl DarwinService {
    /// Creates DarwinService Object
    pub(crate) fn init(home_dir: &Path, executor: Arc<Executor>) -> Result<Box<dyn Service>> {
        let path = home_dir.join(format!(
            "{TERRAINIUMD_DARWIN_SERVICE_PATH}/{}.plist",
            Self::get_name()
        ));

        if !path.parent().unwrap().exists() {
            std::fs::create_dir_all(path.parent().unwrap())
                .expect("failed to create services directory");
        }

        let uid = get_uid(executor.clone())?;

        Ok(Box::new(Self {
            path,
            executor,
            uid,
        }))
    }

    /// Get the service file name.
    ///
    /// Will be different for debug build to avoid interference with release build
    fn get_name() -> String {
        format!("com.csd1100.{}", Self::get_service_identifier())
    }

    fn get_service_identifier() -> &'static str {
        if cfg!(debug_assertions) {
            TERRAINIUMD_DEBUG
        } else {
            TERRAINIUMD
        }
    }

    fn get_target(&self) -> Result<String> {
        Ok(format!("{GUI}/{}", self.uid))
    }

    fn get_service_target(&self) -> Result<String> {
        Ok(format!("{}/{}", self.get_target()?, Self::get_name()))
    }

    fn write_service(&self, enabled: bool) -> Result<()> {
        let service = self.get(enabled)?;
        std::fs::write(&self.path, &service).context("failed to write service")
    }

    /// Runs `launchctl print gui/<uid>/com.csd1100.terrainiumd` command
    /// to get service status.
    fn get_status(&self) -> Result<String> {
        let status = Command::new(
            LAUNCHCTL.to_string(),
            vec![
                PRINT.to_string(),
                self.get_service_target()
                    .context("failed to get service target")?,
            ],
            Some(std::env::temp_dir()),
        );

        let output = self
            .executor
            .get_output(None, status)
            .context("failed to execute the status command")?;

        if !output.status.success() {
            bail!(
                "failed to get status of the service: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        String::from_utf8(output.stdout).context("failed to decode the status")
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use anyhow::Result;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    use crate::client::test_utils::assertions::executor::{AssertExecutor, ExpectedCommand};
    use crate::common::constants::ENABLE;
    use crate::common::execute::MockExecutor;
    use crate::common::types::command::Command;
    use crate::daemon::service::darwin::{
        DarwinService, LAUNCHCTL, LOAD, PRINT, SIGTERM, START, STOP, UNLOAD,
    };
    use crate::daemon::service::tests::Status;
    use crate::daemon::service::{ERROR_SERVICE_NOT_INSTALLED, ERROR_SERVICE_NOT_LOADED};

    fn service_target() -> &'static str {
        if cfg!(debug_assertions) {
            "gui/501/com.csd1100.terrainiumd-debug"
        } else {
            "gui/501/com.csd1100.terrainiumd"
        }
    }

    fn service_file() -> &'static str {
        if cfg!(debug_assertions) {
            "Library/LaunchAgents/com.csd1100.terrainiumd-debug.plist"
        } else {
            "Library/LaunchAgents/com.csd1100.terrainiumd.plist"
        }
    }

    fn expect_is_running(running: bool, executor: MockExecutor) -> MockExecutor {
        AssertExecutor::with(executor)
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(
                        LAUNCHCTL.to_string(),
                        vec![PRINT.to_string(), service_target().to_string()],
                        Some(std::env::temp_dir()),
                    ),
                    exit_code: 0,
                    should_fail_to_execute: false,
                    output: if running {
                        "state = running"
                    } else {
                        "state = not running"
                    }
                    .to_string(),
                },
                1,
            )
            .successfully()
    }

    fn expect_status_mocks(
        home_dir: &Path,
        status: Status,
        is_enabled: bool,
        executor: MockExecutor,
    ) -> MockExecutor {
        match status {
            Status::Running => {
                create_service_file(home_dir, is_enabled).unwrap();
                let executor = expect_is_loaded(true, executor);
                expect_is_running(true, executor)
            }
            Status::NotRunning => {
                create_service_file(home_dir, is_enabled).unwrap();
                let executor = expect_is_loaded(true, executor);
                expect_is_running(false, executor)
            }
            Status::NotLoaded => {
                create_service_file(home_dir, is_enabled).unwrap();
                expect_is_loaded(false, executor)
            }
            Status::NotInstalled => executor,
        }
    }

    fn expect_get_uid() -> MockExecutor {
        AssertExecutor::to()
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(
                        "id".to_string(),
                        vec!["-u".to_string()],
                        Some(std::env::temp_dir()),
                    ),
                    exit_code: 0,
                    should_fail_to_execute: false,
                    output: "501".to_string(),
                },
                1,
            )
            .successfully()
    }

    fn expect_is_loaded(success: bool, executor: MockExecutor) -> MockExecutor {
        AssertExecutor::with(executor)
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(
                        LAUNCHCTL.to_string(),
                        vec![PRINT.to_string(), service_target().to_string()],
                        Some(std::env::temp_dir()),
                    ),
                    exit_code: if success { 0 } else { 1 },
                    should_fail_to_execute: !success,
                    output: "".to_string(),
                },
                1,
            )
            .successfully()
    }

    fn expect_load(home_dir: &Path, executor: MockExecutor) -> MockExecutor {
        AssertExecutor::with(executor)
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(
                        LAUNCHCTL.to_string(),
                        vec![
                            LOAD.to_string(),
                            "gui/501".to_string(),
                            home_dir.join(service_file()).to_string_lossy().to_string(),
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

    fn expect_unload(executor: MockExecutor) -> MockExecutor {
        AssertExecutor::with(executor)
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(
                        LAUNCHCTL.to_string(),
                        vec![UNLOAD.to_string(), service_target().to_string()],
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

    fn expect_enable(executor: MockExecutor) -> MockExecutor {
        AssertExecutor::with(executor)
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(
                        LAUNCHCTL.to_string(),
                        vec![ENABLE.to_string(), service_target().to_string()],
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

    fn expect_start(executor: MockExecutor) -> MockExecutor {
        AssertExecutor::with(executor)
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(
                        LAUNCHCTL.to_string(),
                        vec![START.to_string(), service_target().to_string()],
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
                        LAUNCHCTL.to_string(),
                        vec![
                            STOP.to_string(),
                            SIGTERM.to_string(),
                            service_target().to_string(),
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

    fn create_service_file(home_dir: &Path, is_enabled: bool) -> Result<PathBuf> {
        let service_path = home_dir.join(service_file());
        std::fs::create_dir_all(service_path.parent().unwrap())?;
        let contents = if is_enabled {
            r#"<key>RunAtLoad</key>
        <true/>"#
        } else {
            r#"<key>RunAtLoad</key>
        <false/>"#
        };
        std::fs::write(&service_path, contents)?;
        Ok(service_path)
    }

    #[test]
    fn install_works() -> Result<()> {
        let home_dir = tempdir()?;

        let executor = expect_get_uid();

        // from enable
        let executor = expect_enable(executor);
        let executor = expect_is_loaded(true, executor);
        let executor = expect_unload(executor);
        let executor = expect_is_loaded(false, executor);
        let executor = expect_load(home_dir.path(), executor);

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;
        service.install()?;

        assert!(home_dir.path().join(service_file()).exists());
        assert!(service.is_installed());
        Ok(())
    }

    #[test]
    fn install_loads_if_installed_but_not_loaded() -> Result<()> {
        let home_dir = tempdir()?;

        // installed
        let service_file = create_service_file(home_dir.path(), true)?;

        let executor = expect_get_uid();

        // emulate service is not loaded by returning exit code 1
        let executor = expect_is_loaded(false, executor);
        // load the service
        let executor = expect_load(home_dir.path(), executor);

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;
        service.install()?;

        assert!(service_file.exists());
        assert!(service.is_installed());
        Ok(())
    }

    #[test]
    fn remove_works() -> Result<()> {
        let home_dir = tempdir()?;

        create_service_file(home_dir.path(), true)?;

        let executor = expect_get_uid();
        // emulate service is loaded by returning success
        let executor = expect_is_loaded(true, executor);
        let executor = expect_unload(executor);

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;

        service.remove()?;

        assert!(!service.is_installed());

        Ok(())
    }

    #[test]
    fn remove_throws_error_if_not_installed() -> Result<()> {
        let home_dir = tempdir()?;

        let service = DarwinService::init(home_dir.path(), Arc::new(expect_get_uid()))?;

        let error = service.remove().expect_err("expected error").to_string();

        assert_eq!(error, ERROR_SERVICE_NOT_INSTALLED);

        Ok(())
    }

    #[test]
    fn enable_works() -> Result<()> {
        let home_dir = tempdir()?;

        let service_path = create_service_file(home_dir.path(), true)?;

        let executor = expect_get_uid();
        let executor = expect_is_loaded(false, executor);
        let executor = expect_load(home_dir.path(), executor);
        let executor = expect_enable(executor);

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;
        service.enable(false)?;

        assert!(std::fs::read_to_string(&service_path)?.contains(
            r#"<key>RunAtLoad</key>
        <true/>"#
        ));

        Ok(())
    }

    #[test]
    fn enable_works_with_now() -> Result<()> {
        let home_dir = tempdir()?;

        let service_path = create_service_file(home_dir.path(), true)?;

        // setup mocks
        let executor = expect_get_uid();

        let executor = expect_enable(executor);

        // reload the service
        let executor = expect_is_loaded(true, executor);
        let executor = expect_unload(executor);
        let executor = expect_is_loaded(false, executor);
        let executor = expect_load(home_dir.path(), executor);

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;
        service.enable(true)?;

        assert!(std::fs::read_to_string(&service_path)?.contains(
            r#"<key>RunAtLoad</key>
        <true/>"#
        ));

        Ok(())
    }

    #[test]
    fn enable_throw_error_if_not_installed() -> Result<()> {
        let home_dir = tempdir()?;

        let executor = expect_get_uid();
        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;

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
        let service_path = create_service_file(home_dir.path(), true)?;

        let executor = expect_get_uid();
        let executor = expect_is_loaded(false, executor);
        let executor = expect_load(home_dir.path(), executor);

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;
        service.disable(false)?;

        assert!(std::fs::read_to_string(&service_path)?.contains(
            r#"<key>RunAtLoad</key>
        <false/>"#
        ));
        Ok(())
    }

    #[test]
    fn disable_works_with_now() -> Result<()> {
        let home_dir = tempdir()?;
        let service_path = create_service_file(home_dir.path(), true)?;

        let executor = expect_get_uid();
        let executor = expect_is_loaded(true, executor);
        let executor = expect_is_running(true, executor);
        let executor = expect_stop(executor);

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;
        service.disable(true)?;

        assert!(std::fs::read_to_string(&service_path)?.contains(
            r#"<key>RunAtLoad</key>
        <false/>"#
        ));
        Ok(())
    }

    #[test]
    fn disable_throw_error_if_not_installed() -> Result<()> {
        let home_dir = tempdir()?;

        let executor = expect_get_uid();
        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;

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
        create_service_file(home_dir.path(), true)?;

        let executor = expect_get_uid();
        let executor = expect_is_running(false, executor);
        let executor = expect_start(executor);

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;

        service.start()?;

        Ok(())
    }

    #[test]
    fn start_throws_an_error_if_already_running() -> Result<()> {
        let home_dir = tempdir()?;
        create_service_file(home_dir.path(), true)?;

        let executor = expect_get_uid();
        let executor = expect_is_running(true, executor);

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;

        let error = service.start().expect_err("expected error").to_string();
        assert_eq!(error, "service is already running!");

        Ok(())
    }

    #[test]
    fn start_throws_an_error_if_not_installed() -> Result<()> {
        let home_dir = tempdir()?;

        let executor = expect_get_uid();

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;

        let error = service.start().expect_err("expected error").to_string();
        assert_eq!(error, ERROR_SERVICE_NOT_INSTALLED);

        Ok(())
    }

    #[test]
    fn start_throws_an_error_if_not_loaded() -> Result<()> {
        let home_dir = tempdir()?;
        create_service_file(home_dir.path(), true)?;

        let executor = expect_get_uid();
        let executor = expect_is_loaded(false, executor);

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;

        let error = service.start().expect_err("expected error").to_string();
        assert_eq!(error, ERROR_SERVICE_NOT_LOADED);

        Ok(())
    }

    #[test]
    fn stop_works() -> Result<()> {
        let home_dir = tempdir()?;
        create_service_file(home_dir.path(), true)?;

        let executor = expect_get_uid();
        let executor = expect_is_running(true, executor);
        let executor = expect_stop(executor);

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;

        service.stop()?;

        Ok(())
    }

    #[test]
    fn stop_throws_an_error_if_not_running() -> Result<()> {
        let home_dir = tempdir()?;
        create_service_file(home_dir.path(), true)?;

        let executor = expect_get_uid();
        let executor = expect_is_running(false, executor);

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;

        let error = service.stop().expect_err("expected error").to_string();
        assert_eq!(error, "service is not running!");

        Ok(())
    }

    #[test]
    fn stop_throws_an_error_if_not_installed() -> Result<()> {
        let home_dir = tempdir()?;

        let executor = expect_get_uid();

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;

        let error = service.stop().expect_err("expected error").to_string();
        assert_eq!(error, ERROR_SERVICE_NOT_INSTALLED);

        Ok(())
    }

    #[test]
    fn stop_throws_an_error_if_not_loaded() -> Result<()> {
        let home_dir = tempdir()?;
        create_service_file(home_dir.path(), true)?;

        let executor = expect_get_uid();
        let executor = expect_is_loaded(false, executor);

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;

        let error = service.stop().expect_err("expected error").to_string();
        assert_eq!(error, ERROR_SERVICE_NOT_LOADED);

        Ok(())
    }

    #[test]
    fn reload_works() -> Result<()> {
        let home_dir = tempdir()?;
        create_service_file(home_dir.path(), true)?;

        let executor = expect_get_uid();
        let executor = expect_is_loaded(true, executor);
        let executor = expect_unload(executor);
        let executor = expect_is_loaded(false, executor);
        let executor = expect_load(home_dir.path(), executor);

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;

        service.reload()?;

        Ok(())
    }

    #[test]
    fn reload_throws_not_installed() -> Result<()> {
        let home_dir = tempdir()?;

        let service = DarwinService::init(home_dir.path(), Arc::new(expect_get_uid()))?;

        let error = service.reload().expect_err("expected error").to_string();
        assert_eq!(error, ERROR_SERVICE_NOT_INSTALLED);

        Ok(())
    }

    #[test]
    fn status_not_installed() -> Result<()> {
        let home_dir = tempdir()?;
        let executor = expect_get_uid();
        let executor = expect_status_mocks(home_dir.path(), Status::NotInstalled, false, executor);

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;

        assert_eq!("not installed", service.status()?);
        Ok(())
    }

    #[test]
    fn status_not_loaded() -> Result<()> {
        let home_dir = tempdir()?;
        let executor = expect_get_uid();
        let executor = expect_status_mocks(home_dir.path(), Status::NotLoaded, false, executor);

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;

        assert_eq!("not loaded", service.status()?);
        Ok(())
    }

    #[test]
    fn status_not_running() -> Result<()> {
        let home_dir = tempdir()?;
        let executor = expect_get_uid();
        let executor = expect_status_mocks(home_dir.path(), Status::NotRunning, true, executor);

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;

        assert_eq!("not running (enabled)", service.status()?);
        Ok(())
    }

    #[test]
    fn status_running() -> Result<()> {
        let home_dir = tempdir()?;
        let executor = expect_get_uid();
        let executor = expect_status_mocks(home_dir.path(), Status::Running, false, executor);

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;

        assert_eq!("running (disabled)", service.status()?);
        Ok(())
    }
}
