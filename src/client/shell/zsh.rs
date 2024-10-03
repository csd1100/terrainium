use crate::client::shell::{Shell, Zsh};
use crate::client::types::context::Context;
use crate::client::types::environment::Environment;
use crate::client::types::terrain::Terrain;
use crate::common::constants::{
    FPATH, TERRAIN_INIT_FN, TERRAIN_INIT_SCRIPT, TERRAIN_SELECTED_BIOME, ZSH_ALIASES_TEMPLATE_NAME,
    ZSH_CONSTRUCTORS_TEMPLATE_NAME, ZSH_DESTRUCTORS_TEMPLATE_NAME, ZSH_ENVS_TEMPLATE_NAME,
    ZSH_MAIN_TEMPLATE_NAME,
};
#[double]
use crate::common::execute::CommandToRun;
use crate::common::execute::Execute;
use anyhow::{anyhow, Context as AnyhowContext, Result};
use mockall_double::double;
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};

const ZSH_MAIN_TEMPLATE: &str = include_str!("../../../templates/zsh_final_script.hbs");
const ZSH_ENVS_TEMPLATE: &str = include_str!("../../../templates/zsh_env.hbs");
const ZSH_ALIASES_TEMPLATE: &str = include_str!("../../../templates/zsh_aliases.hbs");
const ZSH_CONSTRUCTORS_TEMPLATE: &str = include_str!("../../../templates/zsh_constructors.hbs");
const ZSH_DESTRUCTORS_TEMPLATE: &str = include_str!("../../../templates/zsh_destructors.hbs");

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

    fn update_rc(_data: String) -> Result<()> {
        todo!()
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
        let mut compiled_script_path: PathBuf = scripts_dir.into();
        compiled_script_path.push(format!("terrain-{}.zwc", biome_name));
        compiled_script_path
    }

    fn script_path(scripts_dir: &Path, biome_name: &String) -> PathBuf {
        let mut script_path: PathBuf = scripts_dir.into();
        script_path.push(format!("terrain-{}.zsh", biome_name));
        script_path
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

        fs::write(script_path, script)
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

    #[cfg(test)]
    pub fn runner_ref(&self) -> &CommandToRun {
        &self.runner
    }
}

#[cfg(test)]
mod test {
    use crate::client::shell::Zsh;
    use crate::client::types::terrain::Terrain;
    use crate::common::execute::MockCommandToRun;
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
}
