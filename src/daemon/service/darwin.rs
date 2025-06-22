use crate::common::constants::{
    get_terrainiumd_pid_file, DISABLE, ENABLE, PATH, TERRAINIUMD_DARWIN_SERVICE_FILE,
};
use crate::common::execute::Execute;
#[mockall_double::double]
use crate::common::execute::Executor;
use crate::common::types::command::Command;
use crate::daemon::service::Service;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const GUI: &str = "gui";
const LAUNCHCTL: &str = "launchctl";
const LOAD: &str = "bootstrap";
const UNLOAD: &str = "bootout";
const PRINT: &str = "print";
const START: &str = "kickstart";
const STOP: &str = "kill";
const PROJECT_ID: &str = "com.csd1100.terrainium";
const SIGTERM: &str = "SIGTERM";

/// Fetches current users id required for `launchctl` commands using
/// `id -u` command.
fn get_uid(executor: Arc<Executor>) -> Result<String> {
    let command = Command::new("id".to_string(), vec!["-u".to_string()], None);
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
    /// `~/Library/LaunchAgents/com.csd1100.terrainium.plist`
    fn is_installed(&self) -> bool {
        self.path.exists()
    }

    /// Copy com.csd1100.terrainium.plist file to `~/Library/LaunchAgents/com.csd1100.terrainium.plist`
    /// and bootstrap the service.
    ///
    /// If `daemon_path` is `Some` then that executable will be launched by `launchd`.
    fn install(&self, daemon_path: Option<PathBuf>) -> Result<()> {
        if self.is_installed() {
            if !self.is_loaded()? {
                println!("loading the service!");
                self.load().context("failed to load the service")?;
            } else {
                println!("service is already installed and loaded!");
            }
            return Ok(());
        }

        let daemon_path =
            daemon_path.unwrap_or(std::env::current_exe().context("failed to get current bin")?);

        let service = self.get(daemon_path)?;
        std::fs::write(&self.path, &service).context("failed to write service")?;

        self.load().context("failed to load service")?;

        Ok(())
    }

    /// Check if service is bootstrapped by using `launchctl print gui/<uid>/com.csd1100.terrainium`
    /// command.
    fn is_loaded(&self) -> Result<bool> {
        if !self.is_installed() {
            bail!(
                "service is not installed, run terrainiumd install-service to install the service"
            );
        }

        let is_bootstrapped = Command::new(
            LAUNCHCTL.to_string(),
            vec![
                PRINT.to_string(),
                self.get_service_target()
                    .context("failed to get service target")?,
            ],
            None,
        );

        let bootstrapped = self
            .executor
            .wait(None, is_bootstrapped, true)
            .context("failed to check if service is installed")?;

        Ok(bootstrapped.success())
    }

    /// Bootstrap using `launchctl` command.
    ///
    /// Even if service is installed the bootstrap command might fail if service
    /// is disabled by the user. User must enable the service to bootstrap and run
    /// the service
    ///
    /// `launchctl bootstrap gui/<uid> ~/Library/LaunchAgents/com.csd1100.terrainium.plist`
    fn load(&self) -> Result<()> {
        if self.is_loaded()? {
            println!("service is already loaded");
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
            None,
        );

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute process")?;

        if !output.status.success() {
            bail!(
                "failed to bootstrap service, enable the service using `terrainiumd enable-service`. error: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    /// Unloads the service.
    ///
    /// `launchctl bootout gui/<uid>/com.csd1100.terrainium`
    fn unload(&self) -> Result<()> {
        if !self.is_loaded()? {
            println!("service is already unloaded");
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
            None,
        );

        let output = self
            .executor
            .get_output(None, command)
            .context("failed to execute process")?;

        if !output.status.success() {
            bail!(
                "failed to bootout service: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Removes the service from `~/Library/LaunchAgents/com.csd1100.terrainium.plist`.
    /// Unload the service if it is loaded.
    fn remove(&self) -> Result<()> {
        self.unload().context("failed to unload")?;
        std::fs::remove_file(&self.path).context("failed to remove service file")
    }

    /// Enables the service to be bootstrapped and start at the login.
    /// The service must be enabled in order to be bootstrapped.
    /// When user is manually starting the service and service is unloaded,
    /// it must be enabled in order to load it again.
    ///
    /// `launchctl enable gui/<uid>/com.csd1100.terrainium`
    ///
    /// If `now` is true then service is loaded and started at the same time.
    fn enable(&self, now: bool) -> Result<()> {
        // enable service
        let command = Command::new(
            LAUNCHCTL.to_string(),
            vec![ENABLE.to_string(), self.get_service_target()?],
            None,
        );

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

        if now {
            // loading will auto-start service due to `RunAtLoad`
            self.load().context("failed to start service")?;
        }

        Ok(())
    }

    /// Disables the service from ever to be loaded until enabled.
    ///
    /// `launchctl disable gui/<uid>/com.csd1100.terrainium`
    fn disable(&self) -> Result<()> {
        // enable service
        let command = Command::new(
            LAUNCHCTL.to_string(),
            vec![DISABLE.to_string(), self.get_service_target()?],
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

    /// Check if terrainiumd process is running by checking status of pid
    /// stored at `/tmp/terrainiumd/pid` file.
    /// Check if service is loaded before and if not, load it.
    fn is_running(&self) -> Result<bool> {
        if !self.is_loaded()? {
            self.load().context("failed to load service")?;
        }

        let pid_file = PathBuf::from(get_terrainiumd_pid_file());
        if !pid_file.exists() {
            return Ok(false);
        }

        let pid =
            std::fs::read_to_string(pid_file).context("failed to read terrainiumd pid file")?;

        let is_running = Command::new("kill".to_string(), vec!["-0".to_string(), pid], None);

        let running = self
            .executor
            .wait(None, is_running, true)
            .context("failed to check if service is running")?;

        Ok(running.success())
    }

    /// Start the service if it is not already running.
    ///
    /// `launchctl kickstart gui/<uid>/com.csd1100.terrainium`
    fn start(&self) -> Result<()> {
        if self.is_running()? {
            bail!("service is already running");
        }

        // start service
        let command = Command::new(
            LAUNCHCTL.to_string(),
            vec![START.to_string(), self.get_service_target()?],
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

    /// Stops the service process by sending it `SIGTERM`.
    ///
    /// `launchctl kill SIGTERM gui/<uid>/com.csd1100.terrainium`
    fn stop(&self) -> Result<()> {
        if !self.is_running()? {
            bail!("service is not running");
        }

        // stop service
        let command = Command::new(
            LAUNCHCTL.to_string(),
            vec![
                STOP.to_string(),
                SIGTERM.to_string(),
                self.get_service_target()?,
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

    /// Returns service `plist` file contents.
    /// `daemon_path` will be executable to run.
    fn get(&self, daemon_path: PathBuf) -> Result<String> {
        if !daemon_path.exists() {
            bail!("{} does not exist", daemon_path.display());
        }

        let service = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
    <dict>
        <key>Label</key>
        <string>{PROJECT_ID}</string>
        <key>ProgramArguments</key>
        <array>
            <string>{}</string>
            <string>--force</string>
        </array>
        <key>EnvironmentVariables</key>
        <dict>
            <key>PATH</key>
            <string>{}</string>
        </dict>
        <key>RunAtLoad</key>
        <true/>
        <key>StandardOutPath</key>
        <string>/tmp/terrainiumd.stdout.log</string>
        <key>StandardErrorPath</key>
        <string>/tmp/terrainiumd.stderr.log</string>
        <key>ProcessType</key>
        <string>Background</string>
    </dict>
</plist>"#,
            daemon_path.display(),
            std::env::var(PATH).context("failed to get PATH")?,
        );
        Ok(service)
    }
}

impl DarwinService {
    /// Creates DarwinService Object with passed service file created using
    /// passed in `home_dir`
    pub(crate) fn init(home_dir: &Path, executor: Arc<Executor>) -> Result<Box<dyn Service>> {
        let path = home_dir.join(TERRAINIUMD_DARWIN_SERVICE_FILE);

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

    fn get_target(&self) -> Result<String> {
        Ok(format!("{GUI}/{}", self.uid))
    }

    fn get_service_target(&self) -> Result<String> {
        Ok(format!("{}/{PROJECT_ID}", self.get_target()?))
    }
}

#[cfg(test)]
mod tests {
    use crate::client::test_utils::assertions::executor::{AssertExecutor, ExpectedCommand};
    use crate::client::test_utils::{restore_env_var, set_env_var};
    use crate::common::constants::{PATH, TERRAINIUMD_DARWIN_SERVICE_FILE};
    use crate::common::execute::MockExecutor;
    use crate::common::types::command::Command;
    use crate::daemon::service::darwin::{
        DarwinService, LAUNCHCTL, LOAD, PRINT, PROJECT_ID, UNLOAD,
    };
    use crate::daemon::service::tests::{cleanup_test_daemon_binary, create_test_daemon_binary};
    use anyhow::Result;
    use serial_test::serial;
    use std::env::VarError;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use tempfile::tempdir;

    fn expected_get_uid() -> MockExecutor {
        AssertExecutor::to()
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new("id".to_string(), vec!["-u".to_string()], None),
                    exit_code: 0,
                    should_error: false,
                    output: "501".to_string(),
                },
                1,
            )
            .successfully()
    }

    fn expected_load_commands(home_dir: &Path) -> MockExecutor {
        let executor = expected_get_uid();

        let executor = AssertExecutor::with(executor).wait_for(
            None,
            ExpectedCommand {
                command: Command::new(
                    LAUNCHCTL.to_string(),
                    vec![PRINT.to_string(), format!("gui/501/{PROJECT_ID}")],
                    None,
                ),
                exit_code: 1,
                should_error: false,
                output: "".to_string(),
            },
            true,
            1,
        );

        AssertExecutor::with(executor)
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(
                        LAUNCHCTL.to_string(),
                        vec![
                            LOAD.to_string(),
                            "gui/501".to_string(),
                            home_dir
                                .join(TERRAINIUMD_DARWIN_SERVICE_FILE)
                                .to_string_lossy()
                                .to_string(),
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

    fn expected_unload_commands() -> MockExecutor {
        let executor = expected_get_uid();

        let executor = AssertExecutor::with(executor).wait_for(
            None,
            ExpectedCommand {
                command: Command::new(
                    LAUNCHCTL.to_string(),
                    vec![PRINT.to_string(), format!("gui/501/{PROJECT_ID}")],
                    None,
                ),
                exit_code: 0,
                should_error: false,
                output: "".to_string(),
            },
            true,
            1,
        );

        AssertExecutor::with(executor)
            .get_output_for(
                None,
                ExpectedCommand {
                    command: Command::new(
                        LAUNCHCTL.to_string(),
                        vec![UNLOAD.to_string(), format!("gui/501/{PROJECT_ID}")],
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

    #[test]
    fn install_works() -> Result<()> {
        let home_dir = tempdir()?;

        let service = DarwinService::init(
            home_dir.path(),
            Arc::new(expected_load_commands(home_dir.path())),
        )?;
        service.install(None)?;
        assert!(home_dir
            .path()
            .join(TERRAINIUMD_DARWIN_SERVICE_FILE)
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

        // create daemon file

        let service = DarwinService::init(
            home_dir.path(),
            Arc::new(expected_load_commands(home_dir.path())),
        )?;

        service.install(Some(create_test_daemon_binary()?))?;
        assert!(home_dir
            .path()
            .join(TERRAINIUMD_DARWIN_SERVICE_FILE)
            .exists());
        assert!(service.is_installed());

        let contents =
            std::fs::read_to_string(home_dir.path().join(TERRAINIUMD_DARWIN_SERVICE_FILE))?;
        let expected = std::fs::read_to_string("./tests/data/com.csd1100.terrainium.plist")?;

        assert_eq!(contents, expected);

        cleanup_test_daemon_binary()?;
        unsafe { restore_env_var(PATH, path) }
        Ok(())
    }

    #[test]
    fn install_with_daemon_path_errors_no_daemon() -> Result<()> {
        let home_dir = tempdir()?;

        let executor = expected_get_uid();

        let service = DarwinService::init(home_dir.path(), Arc::new(executor))?;
        let error = service
            .install(Some(PathBuf::from("/non_existent")))
            .expect_err("expected error")
            .to_string();

        assert_eq!(error, "/non_existent does not exist");
        Ok(())
    }

    #[test]
    fn remove_works() -> Result<()> {
        let home_dir = tempdir()?;

        let service_path = home_dir.path().join(TERRAINIUMD_DARWIN_SERVICE_FILE);
        std::fs::create_dir_all(service_path.parent().unwrap())?;
        std::fs::write(&service_path, "")?;

        let service = DarwinService::init(home_dir.path(), Arc::new(expected_unload_commands()))?;

        service.remove()?;

        assert!(!service.is_installed());

        Ok(())
    }
}
