use crate::client::shell::{Shell, Zsh};
use crate::client::types::context::Context;
use crate::client::types::environment::Environment;
use crate::client::types::terrain::Terrain;
use crate::common::constants::{
    FPATH, TERRAIN_INIT_FN, TERRAIN_INIT_SCRIPT, TERRAIN_SELECTED_BIOME, ZSH_ALIASES_TEMPLATE_NAME,
    ZSH_CONSTRUCTORS_TEMPLATE_NAME, ZSH_DESTRUCTORS_TEMPLATE_NAME, ZSH_ENVS_TEMPLATE_NAME,
    ZSH_MAIN_TEMPLATE_NAME,
};
#[mockall_double::double]
use crate::common::execute::CommandToRun;
use crate::common::execute::Execute;
use anyhow::{anyhow, Context as AnyhowContext, Result};
use home::home_dir;
use std::collections::BTreeMap;
use std::fs;
use std::fs::{copy, exists, read_to_string, remove_file, write};
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};

const ZSH_MAIN_TEMPLATE: &str = include_str!("../../../templates/zsh_final_script.hbs");
const ZSH_ENVS_TEMPLATE: &str = include_str!("../../../templates/zsh_env.hbs");
const ZSH_ALIASES_TEMPLATE: &str = include_str!("../../../templates/zsh_aliases.hbs");
const ZSH_CONSTRUCTORS_TEMPLATE: &str = include_str!("../../../templates/zsh_constructors.hbs");
const ZSH_DESTRUCTORS_TEMPLATE: &str = include_str!("../../../templates/zsh_destructors.hbs");

pub const ZSH_INIT_SCRIPT_NAME: &str = "terrainium_init.zsh";
const INIT_SCRIPT: &str = include_str!("../../scripts/terrainium_init.zsh");

impl Shell for Zsh {
    fn get() -> Self {
        Self {
            exe: "/bin/zsh".to_string(),
            runner: CommandToRun::new("/bin/zsh".to_string(), vec![], None),
        }
    }

    fn runner(&self) -> CommandToRun {
        self.runner.clone()
    }

    fn get_init_rc_contents() -> String {
        format!(
            r#"
source "$HOME/.config/terrainium/shell_integration/{}"
"#,
            ZSH_INIT_SCRIPT_NAME
        )
    }

    fn setup_integration(&self, init_script_dir: &Path) -> Result<()> {
        let init_script_location = init_script_dir.join(ZSH_INIT_SCRIPT_NAME);

        if !exists(&init_script_location).expect("failed to check if init-script exists") {
            println!("WARNING - init-script not found in config directory, copying script to config directory");
            write(&init_script_location, INIT_SCRIPT).expect("failed to create init-script file");
        } else if read_to_string(&init_script_location).expect("failed to read init-script")
            != INIT_SCRIPT
        {
            let mut backup = init_script_location.clone();
            backup.set_extension("zsh.bkp");

            copy(&init_script_location, backup).expect("failed to remove init-script");
            remove_file(&init_script_location).expect("failed to remove init-script");
            println!("WARNING - init-script was outdated in config directory, copying newer script to config directory");
            write(&init_script_location, INIT_SCRIPT).expect("failed to create init-script file");
        }

        let mut compiled_path = init_script_location.clone();
        compiled_path.set_extension("zsh.zwc");

        self.compile_script(&init_script_location, &compiled_path)
    }

    fn update_rc(&self, path: Option<PathBuf>) -> Result<()> {
        let path = path.unwrap_or_else(|| home_dir().expect("cannot get home dir").join(".zshrc"));
        let rc = read_to_string(&path).context("failed to read rc")?;
        if !rc.contains(&Self::get_init_rc_contents()) {
            let rc_file = fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .context("failed to open rc");
            rc_file?
                .write_all(Self::get_init_rc_contents().as_bytes())
                .context("failed to write rc")?;
        }
        Ok(())
    }

    fn generate_scripts(&self, context: &Context, terrain: Terrain) -> Result<()> {
        let scripts_dir = context.scripts_dir();

        let result: Result<Vec<_>> = terrain
            .biomes()
            .keys()
            .map(|biome_name| -> Result<()> {
                self.create_and_compile(&terrain, &scripts_dir, biome_name.to_string())?;
                Ok(())
            })
            .collect();
        result?;

        self.create_and_compile(&terrain, &scripts_dir, "none".to_string())?;

        Ok(())
    }

    fn execute(
        &self,
        mut args: Vec<String>,
        envs: Option<BTreeMap<String, String>>,
    ) -> Result<Output> {
        let mut final_args = vec!["-c".to_string()];
        final_args.append(&mut args);

        let mut runner = self.runner();
        runner.set_args(final_args);
        runner.set_envs(envs);

        runner.get_output()
    }

    async fn spawn(&self, envs: BTreeMap<String, String>) -> Result<ExitStatus> {
        let mut runner = self.runner();
        runner.set_args(vec!["-i".to_string(), "-s".to_string()]);
        runner.set_envs(Some(envs));

        runner.async_spawn().await.context("Failed to run zsh")
    }

    fn generate_envs(&self, context: &Context, biome: String) -> Result<BTreeMap<String, String>> {
        let scripts_dir = context.scripts_dir();
        let compiled_script = Self::compiled_script_path(&scripts_dir, &biome)
            .to_str()
            .expect("path to be converted to string")
            .to_string();

        let mut envs = BTreeMap::new();

        let updated_fpath = format!("{}:{}", compiled_script, self.get_fpath()?);
        envs.insert(FPATH.to_string(), updated_fpath);
        envs.insert(TERRAIN_INIT_SCRIPT.to_string(), compiled_script);
        envs.insert(
            TERRAIN_INIT_FN.to_string(),
            format!("terrain-{}.zsh", biome),
        );
        envs.insert(TERRAIN_SELECTED_BIOME.to_string(), biome);

        Ok(envs)
    }

    fn templates() -> BTreeMap<String, String> {
        let mut templates: BTreeMap<String, String> = BTreeMap::new();
        templates.insert(
            ZSH_MAIN_TEMPLATE_NAME.to_string(),
            ZSH_MAIN_TEMPLATE.to_string(),
        );
        templates.insert(
            ZSH_ENVS_TEMPLATE_NAME.to_string(),
            ZSH_ENVS_TEMPLATE.to_string(),
        );
        templates.insert(
            ZSH_ALIASES_TEMPLATE_NAME.to_string(),
            ZSH_ALIASES_TEMPLATE.to_string(),
        );
        templates.insert(
            ZSH_CONSTRUCTORS_TEMPLATE_NAME.to_string(),
            ZSH_CONSTRUCTORS_TEMPLATE.to_string(),
        );
        templates.insert(
            ZSH_DESTRUCTORS_TEMPLATE_NAME.to_string(),
            ZSH_DESTRUCTORS_TEMPLATE.to_string(),
        );
        templates
    }
}

impl Zsh {
    fn get_fpath(&self) -> Result<String> {
        let command = "/bin/echo -n $FPATH";
        let args = vec!["-c".to_string(), command.to_string()];

        let mut runner = self.runner();
        runner.set_args(args);

        let output = runner.get_output().context("failed to get fpath")?.stdout;
        String::from_utf8(output).context("failed to convert stdout to string")
    }

    fn create_and_compile(
        &self,
        terrain: &Terrain,
        scripts_dir: &Path,
        biome_name: String,
    ) -> Result<()> {
        let script_path = Self::script_path(scripts_dir, &biome_name);
        self.create_script(terrain, Some(biome_name.to_string()), &script_path)?;

        let compiled_script_path = Self::compiled_script_path(scripts_dir, &biome_name);
        self.compile_script(&script_path, &compiled_script_path)?;

        Ok(())
    }

    fn compiled_script_path(scripts_dir: &Path, biome_name: &String) -> PathBuf {
        scripts_dir.join(format!("terrain-{}.zwc", biome_name))
    }

    fn script_path(scripts_dir: &Path, biome_name: &String) -> PathBuf {
        scripts_dir.join(format!("terrain-{}.zsh", biome_name))
    }

    fn create_script(
        &self,
        terrain: &Terrain,
        biome_name: Option<String>,
        script_path: &PathBuf,
    ) -> Result<()> {
        let environment = Environment::from(terrain, biome_name.clone()).unwrap_or_else(|_| {
            panic!(
                "expected to generate environment from terrain for biome {:?}",
                biome_name
            )
        });

        let script = environment
            .to_rendered(ZSH_MAIN_TEMPLATE_NAME.to_string(), Zsh::templates())
            .unwrap_or_else(|_| panic!("script to be rendered for biome: {:?}", biome_name));

        write(script_path, script)
            .context(format!("failed to write script to path {:?}", script_path))?;

        Ok(())
    }

    fn compile_script(&self, script_path: &Path, compiled_script_path: &Path) -> Result<()> {
        let args = vec![format!(
            "zcompile -URz {} {}",
            compiled_script_path.to_string_lossy(),
            script_path.to_string_lossy()
        )];

        let output = self
            .execute(args, None)
            .context(format!("failed to compile script {:?}", script_path))?;

        if output.status.into_raw() != 0 {
            return Err(anyhow!(
                "compiling script failed with exit code {:?}\n error: {}",
                output.status.into_raw(),
                std::str::from_utf8(&output.stderr).expect("failed to convert STDERR to string")
            ));
        }

        Ok(())
    }

    #[cfg(test)]
    pub fn build(runner: CommandToRun) -> Self {
        Self {
            exe: "/bin/zsh".to_string(),
            runner,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::client::shell::{Shell, Zsh};
    use crate::client::types::terrain::Terrain;
    use crate::client::utils::ExpectShell;
    use crate::common::execute::MockCommandToRun;
    use serial_test::serial;
    use std::fs;
    use std::fs::{create_dir_all, exists, write};
    use std::os::unix::process::ExitStatusExt;
    use std::path::PathBuf;
    use std::process::{ExitStatus, Output};

    #[should_panic(
        expected = "expected to generate environment from terrain for biome Some(\"invalid_biome_name\")"
    )]
    #[test]
    fn create_script_will_panic_if_invalid_biome_name() {
        let zsh = Zsh::build(MockCommandToRun::default());

        let terrain = Terrain::example();

        zsh.create_script(
            &terrain,
            Some("invalid_biome_name".to_string()),
            &PathBuf::new(),
        )
        .expect("error not to be thrown");
    }

    #[test]
    fn compile_script_should_return_error_if_non_zero_exit_code() {
        let mut mock_wait = MockCommandToRun::default();
        mock_wait.expect_get_output().times(1).return_once(|| {
            Ok(Output {
                status: ExitStatus::from_raw(1),
                stdout: Vec::<u8>::new(),
                stderr: Vec::<u8>::from("some error while compiling"),
            })
        });

        mock_wait
            .expect_set_args()
            .withf(|_| true)
            .return_once(|_| ());
        mock_wait
            .expect_set_envs()
            .withf(|_| true)
            .return_once(|_| ());

        let mut mock_run = MockCommandToRun::default();
        mock_run.expect_clone().times(1).return_once(|| mock_wait);
        let zsh = Zsh::build(mock_run);

        let err = zsh
            .compile_script(PathBuf::new().as_path(), PathBuf::new().as_path())
            .expect_err("error to be thrown");

        assert_eq!(
            "compiling script failed with exit code 1\n error: some error while compiling",
            err.to_string()
        );
    }

    #[test]
    fn update_rc_path() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(temp_dir.path().join(".zshrc"), "").unwrap();
        Zsh::build(MockCommandToRun::default())
            .update_rc(Some(temp_dir.path().join(".zshrc")))
            .unwrap();

        let expected =
            "\nsource \"$HOME/.config/terrainium/shell_integration/terrainium_init.zsh\"\n";
        assert_eq!(
            expected,
            fs::read_to_string(temp_dir.path().join(".zshrc")).unwrap()
        );
    }

    #[serial]
    #[test]
    fn shell_integration() -> anyhow::Result<()> {
        let home_dir = tempfile::tempdir()?;

        let zsh_integration_script_location =
            home_dir.path().join(".config/terrainium/shell_integration");
        create_dir_all(&zsh_integration_script_location)?;

        let mut zsh_integration_script = zsh_integration_script_location.clone();
        zsh_integration_script.push("terrainium_init.zsh");

        let mut compiled_zsh_integration_script = zsh_integration_script.clone();
        compiled_zsh_integration_script.set_extension("zsh.zwc");

        let expected_shell_operation = ExpectShell::to()
            .compile_script_for(&zsh_integration_script, &compiled_zsh_integration_script)
            .successfully();

        Zsh::build(expected_shell_operation)
            .setup_integration(&zsh_integration_script_location)
            .expect("to succeed");

        assert!(exists(zsh_integration_script)
            .expect("failed to check if shell integration script created"));

        Ok(())
    }

    #[serial]
    #[test]
    fn shell_integration_replace() -> anyhow::Result<()> {
        let home_dir = tempfile::tempdir()?;

        let zsh_integration_script_location =
            home_dir.path().join(".config/terrainium/shell_integration");
        create_dir_all(&zsh_integration_script_location)?;

        let mut zsh_integration_script = zsh_integration_script_location.clone();
        zsh_integration_script.push("terrainium_init.zsh");

        let mut zsh_integration_script_backup = zsh_integration_script.clone();
        zsh_integration_script_backup.set_extension("zsh.bkp");

        let mut compiled_zsh_integration_script = zsh_integration_script.clone();
        compiled_zsh_integration_script.set_extension("zsh.zwc");

        write(
            home_dir
                .path()
                .join(".config/terrainium/shell_integration/terrainium_init.zsh"),
            "",
        )
        .expect("test shell integration to be written");

        let expected_shell_operation = ExpectShell::to()
            .compile_script_for(&zsh_integration_script, &compiled_zsh_integration_script)
            .successfully();

        Zsh::build(expected_shell_operation)
            .setup_integration(&zsh_integration_script_location)
            .expect("to succeed");

        assert!(exists(zsh_integration_script)
            .expect("failed to check if shell integration script created"));

        assert!(exists(zsh_integration_script_backup)
            .expect("failed to check if shell integration script created"));

        Ok(())
    }
}
