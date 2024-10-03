use crate::client::shell::{Shell, Zsh};
#[double]
use crate::client::types::client::Client;
use crate::common::constants::{
    TERRAINIUM_DEV, TERRAINIUM_ENABLED, TERRAINIUM_EXECUTABLE, TERRAINIUM_SESSION_ID, TERRAIN_DIR,
    TERRAIN_SELECTED_BIOME,
};
use anyhow::{anyhow, Result};
use home::home_dir;
use mockall_double::double;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{env, fs};
use uuid::Uuid;

#[derive(Debug)]
pub struct Context {
    session_id: String,
    current_dir: PathBuf,
    central_dir: PathBuf,
    selected_biome: String,
    shell: Zsh,
    client: Option<Client>,
}

impl Context {}

const TERRAIN_TOML: &str = "terrain.toml";
const CONFIG_LOCATION: &str = ".config/terrainium";
const TERRAINS_DIR_NAME: &str = "terrains";
const SCRIPTS_DIR_NAME: &str = "scripts";

impl Default for Context {
    fn default() -> Self {
        Self::generate(None)
    }
}

impl Context {
    pub fn generate(client: Option<Client>) -> Self {
        let session_id =
            env::var(TERRAINIUM_SESSION_ID).unwrap_or_else(|_| Uuid::new_v4().to_string());

        let selected_biome = env::var(TERRAIN_SELECTED_BIOME).unwrap_or_else(|_| "".to_string());

        Context {
            session_id,
            current_dir: env::current_dir().expect("failed to get current directory"),
            central_dir: get_central_dir_location(
                env::current_dir().expect("failed to get current directory"),
            ),
            selected_biome,
            shell: Zsh::get(),
            client,
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn selected_biome(&self) -> &str {
        &self.selected_biome
    }

    pub fn current_dir(&self) -> &PathBuf {
        &self.current_dir
    }

    pub fn central_dir(&self) -> &PathBuf {
        &self.central_dir
    }

    pub fn name(&self) -> String {
        self.current_dir
            .file_name()
            .expect("failed to get current directory name")
            .to_str()
            .expect("failed to convert directory name to string")
            .to_string()
    }

    pub fn toml_exists(&self) -> bool {
        fs::exists(self.local_toml_path()).expect("failed to check if local terrain.toml exists")
            || fs::exists(self.central_toml_path())
                .expect("failed to check if central terrain.toml exists")
    }

    pub fn new_toml_path(&self, central: bool) -> PathBuf {
        if central {
            self.central_toml_path()
        } else {
            self.local_toml_path()
        }
    }

    pub fn toml_path(&self) -> Result<PathBuf> {
        if fs::exists(self.local_toml_path()).expect("failed to check if local terrain.toml exists")
        {
            Ok(self.local_toml_path())
        } else if fs::exists(self.central_toml_path())
            .expect("failed to check if central terrain.toml exists")
        {
            Ok(self.central_toml_path())
        } else {
            Err(anyhow!(
                "terrain.toml does not exists, run `terrainium init` to initialize terrain."
            ))
        }
    }

    pub fn local_toml_path(&self) -> PathBuf {
        let mut base_dir: PathBuf = self.current_dir.clone();
        base_dir.push(TERRAIN_TOML);
        base_dir
    }

    pub fn central_toml_path(&self) -> PathBuf {
        let mut base_dir: PathBuf = self.central_dir.clone();
        base_dir.push(TERRAIN_TOML);
        base_dir
    }

    pub fn scripts_dir(&self) -> PathBuf {
        let mut scripts_dir = self.central_dir.clone();
        scripts_dir.push(SCRIPTS_DIR_NAME);
        scripts_dir
    }

    pub(crate) fn shell(&self) -> &Zsh {
        &self.shell
    }

    pub fn set_client(&mut self, client: Client) {
        self.client = Some(client);
    }

    pub(crate) fn socket(&mut self) -> &mut Client {
        self.client.as_mut().expect("failed to get socket client")
    }

    pub(crate) fn terrainium_envs(&self) -> BTreeMap<String, String> {
        let mut terrainium_envs = BTreeMap::<String, String>::new();
        terrainium_envs.insert(
            TERRAIN_DIR.to_string(),
            self.current_dir().to_string_lossy().to_string(),
        );
        terrainium_envs.insert(
            TERRAINIUM_SESSION_ID.to_string(),
            self.session_id().to_string(),
        );
        terrainium_envs.insert(TERRAINIUM_ENABLED.to_string(), "true".to_string());

        if self.name() == "terrainium"
            && env::var(TERRAINIUM_DEV).unwrap_or_else(|_| "false".to_string()) == "true"
        {
            let exe = self.current_dir().join("target/debug/terrainium");
            terrainium_envs.insert(TERRAINIUM_EXECUTABLE.to_string(), exe.display().to_string());
        }

        terrainium_envs
    }

    #[cfg(test)]
    pub(crate) fn build(
        current_dir: PathBuf,
        central_dir: PathBuf,
        shell: Zsh,
        socket: Option<Client>,
    ) -> Self {
        Context {
            session_id: "some".to_string(),
            current_dir,
            central_dir,
            selected_biome: "".to_string(),
            shell,
            client: socket,
        }
    }

    #[cfg(test)]
    pub(crate) fn build_without_paths(shell: Zsh, socket: Option<Client>) -> Self {
        Context {
            session_id: "some".to_string(),
            current_dir: env::current_dir().expect("failed to get current directory"),
            central_dir: get_central_dir_location(
                env::current_dir().expect("failed to get current directory"),
            ),
            selected_biome: "".to_string(),
            shell,
            client: socket,
        }
    }
}

fn get_central_dir_location(current_dir: PathBuf) -> PathBuf {
    let mut central_dir = home_dir().expect("failed to get home directory");
    central_dir.push(CONFIG_LOCATION);
    central_dir.push(TERRAINS_DIR_NAME);

    let terrain_dir_name = Path::canonicalize(current_dir.as_path())
        .expect("expected current directory to be valid")
        .to_string_lossy()
        .to_string()
        .replace('/', "_");
    central_dir.push(terrain_dir_name);

    central_dir
}

#[cfg(test)]
mod test {
    use super::Context;
    use crate::client::shell::Zsh;
    use crate::common::constants::{TERRAINIUM_ENABLED, TERRAINIUM_SESSION_ID};
    use crate::common::execute::MockCommandToRun;
    use anyhow::Result;
    use home::home_dir;
    use serial_test::serial;
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};
    use std::{env, fs};
    use tempfile::tempdir;

    #[serial]
    #[test]
    fn new_creates_context() -> Result<()> {
        let current_dir = env::current_dir().expect("failed to get current directory");
        let central_dir = get_central_dir_location();

        let new_mock = MockCommandToRun::new_context();
        new_mock
            .expect()
            .withf(|_, _, _| true)
            .times(1)
            .returning(|_, _, _| MockCommandToRun::default());

        let actual = Context::generate(None);
        assert_eq!(current_dir, actual.current_dir);
        assert_eq!(central_dir, actual.central_dir);

        Ok(())
    }

    #[test]
    fn current_dir_returns_current_dir() -> Result<()> {
        let context = Context::build_without_paths(Zsh::build(MockCommandToRun::default()), None);
        assert_eq!(
            &env::current_dir().expect("failed to get current directory"),
            context.current_dir()
        );
        Ok(())
    }

    #[test]
    fn terrainium_envs() -> Result<()> {
        let mut expected_map = BTreeMap::<String, String>::new();
        expected_map.insert(
            "TERRAIN_DIR".to_string(),
            env::current_dir()
                .expect("to be found")
                .display()
                .to_string(),
        );
        expected_map.insert(TERRAINIUM_ENABLED.to_string(), "true".to_string());

        let context = Context::build_without_paths(Zsh::build(MockCommandToRun::default()), None);

        assert!(context
            .terrainium_envs()
            .contains_key(TERRAINIUM_SESSION_ID));

        context
            .terrainium_envs()
            .iter()
            .filter(|(key, _)| *key != TERRAINIUM_SESSION_ID)
            .for_each(|(key, value)| {
                assert_eq!(value, expected_map.get(key).expect("to be present"));
            });

        Ok(())
    }

    #[test]
    fn local_toml_path_return_current_terrain_toml_path() -> Result<()> {
        let mut expected_terrain_toml =
            env::current_dir().expect("failed to get current directory");
        expected_terrain_toml.push("terrain.toml");

        let context = Context::build_without_paths(Zsh::build(MockCommandToRun::default()), None);

        assert_eq!(context.local_toml_path(), expected_terrain_toml);

        Ok(())
    }

    #[test]
    fn toml_path_return_current_terrain_toml_path() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let mut expected_terrain_toml: PathBuf = current_dir.path().into();
        expected_terrain_toml.push("terrain.toml");

        fs::write(&expected_terrain_toml, "").expect("to create test terrain.toml");

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        assert_eq!(
            expected_terrain_toml,
            context.toml_path().expect("to return valid value")
        );

        current_dir
            .close()
            .expect("test directory to be cleaned up");
        central_dir
            .close()
            .expect("test directory to be cleaned up");

        Ok(())
    }

    #[test]
    fn toml_path_return_central_terrain_toml_path() -> Result<()> {
        let central_dir = tempdir()?;
        let current_dir = tempdir()?;

        let mut expected_terrain_toml: PathBuf = central_dir.path().into();
        expected_terrain_toml.push("terrain.toml");
        fs::write(&expected_terrain_toml, "").expect("to create test terrain.toml");

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        assert_eq!(
            expected_terrain_toml,
            context.toml_path().expect("to return valid value")
        );

        current_dir
            .close()
            .expect("test directory to be cleaned up");
        central_dir
            .close()
            .expect("test directory to be cleaned up");

        Ok(())
    }

    #[test]
    fn central_toml_path_return_central_terrain_toml_path() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let mut expected_terrain_toml: PathBuf = central_dir.path().into();
        expected_terrain_toml.push("terrain.toml");

        fs::write(&expected_terrain_toml, "").expect("to create test terrain.toml");

        let context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(MockCommandToRun::default()),
            None,
        );

        assert_eq!(context.central_toml_path(), expected_terrain_toml);

        current_dir
            .close()
            .expect("test directory to be cleaned up");
        central_dir
            .close()
            .expect("test directory to be cleaned up");

        Ok(())
    }

    #[test]
    fn toml_path_returns_error_if_does_not_exists() -> Result<()> {
        let central_dir = tempdir()?;
        let current_dir = tempdir()?;

        let err = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(MockCommandToRun::default()),
            None,
        )
        .toml_path()
        .expect_err("to error to be thrown")
        .to_string();

        assert_eq!(
            "terrain.toml does not exists, run `terrainium init` to initialize terrain.",
            err
        );

        current_dir
            .close()
            .expect("test directory to be cleaned up");
        central_dir
            .close()
            .expect("test directory to be cleaned up");

        Ok(())
    }

    #[test]
    fn central_dir_returns_config_location() -> Result<()> {
        let context = Context::build_without_paths(Zsh::build(MockCommandToRun::default()), None);
        let central_dir = get_central_dir_location();

        assert_eq!(&central_dir, context.central_dir());
        Ok(())
    }

    #[test]
    fn scripts_dir_returns_scripts_location() -> Result<()> {
        let context = Context::build_without_paths(Zsh::build(MockCommandToRun::default()), None);
        let mut scripts_dir = get_central_dir_location();
        scripts_dir.push("scripts");

        assert_eq!(scripts_dir, context.scripts_dir());
        Ok(())
    }

    #[test]
    fn name_return_current_dir_name() {
        let context = Context::build_without_paths(Zsh::build(MockCommandToRun::default()), None);
        assert_eq!(context.name(), "terrainium");
    }

    // #[test]
    // fn socket_return_socket_witout_panic() {
    //     let mut context = Context::build_without_paths(
    //         Zsh::build(MockCommandToRun::default()),
    //         Some(MockClient::default()),
    //     );
    //     context.socket();
    // }

    #[should_panic]
    #[test]
    fn socket_panic() {
        let mut context =
            Context::build_without_paths(Zsh::build(MockCommandToRun::default()), None);
        context.socket();
    }

    fn get_central_dir_location() -> PathBuf {
        let current_dir = env::current_dir().expect("failed to get current directory");
        let terrain_dir_name = Path::canonicalize(current_dir.as_path())
            .expect("current directory to be present")
            .to_string_lossy()
            .to_string()
            .replace('/', "_");

        let mut central_dir = home_dir().expect("failed to get home directory");
        central_dir.push(".config/terrainium/terrains");
        central_dir.push(terrain_dir_name);

        central_dir
    }
}
