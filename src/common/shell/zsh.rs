use crate::client::types::context::Context;
use crate::common::constants::{
    ZSH_ALIASES_TEMPLATE_NAME, ZSH_CONSTRUCTORS_TEMPLATE_NAME, ZSH_DESTRUCTORS_TEMPLATE_NAME,
    ZSH_ENVS_TEMPLATE_NAME, ZSH_MAIN_TEMPLATE_NAME,
};

#[double]
use crate::common::execute::Run;
use crate::common::shell::{Shell, Zsh};
use crate::common::types::environment::Environment;
use crate::common::types::terrain::Terrain;
use anyhow::{anyhow, Context as AnyhowContext, Result};
use mockall_double::double;
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Output;

const ZSH_MAIN_TEMPLATE: &str = include_str!("../../../templates/zsh_final_script.hbs");
const ZSH_ENVS_TEMPLATE: &str = include_str!("../../../templates/zsh_env.hbs");
const ZSH_ALIASES_TEMPLATE: &str = include_str!("../../../templates/zsh_aliases.hbs");
const ZSH_CONSTRUCTORS_TEMPLATE: &str = include_str!("../../../templates/zsh_constructors.hbs");
const ZSH_DESTRUCTORS_TEMPLATE: &str = include_str!("../../../templates/zsh_destructors.hbs");

impl Shell for Zsh {
    fn get() -> Self {
        Self {
            exe: "/bin/zsh".to_string(),
            runner: Run::new("/bin/zsh".to_string(), vec![], None),
        }
    }

    fn exe(&self) -> String {
        self.exe.clone()
    }

    fn runner(&self) -> Run {
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
    fn create_and_compile(
        &self,
        terrain: &Terrain,
        scripts_dir: &Path,
        biome_name: String,
    ) -> Result<()> {
        let mut script_path: PathBuf = scripts_dir.into();
        script_path.push(format!("terrain-{}.zsh", biome_name));
        self.create_script(terrain, Some(biome_name.to_string()), &script_path)?;

        let mut compiled_script_path: PathBuf = scripts_dir.into();
        compiled_script_path.push(format!("terrain-{}.zwc", biome_name));
        self.compile_script(&script_path, &compiled_script_path)?;

        Ok(())
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
    pub fn build(runner: Run) -> Self {
        Self {
            exe: "/bin/zsh".to_string(),
            runner,
        }
    }

    #[cfg(test)]
    pub fn runner_ref(&self) -> &Run {
        &self.runner
    }
}

#[cfg(test)]
mod test {
    use crate::common::execute::MockRun;
    use crate::common::shell::Zsh;
    use crate::common::types::terrain::Terrain;
    use std::os::unix::process::ExitStatusExt;
    use std::path::PathBuf;
    use std::process::{ExitStatus, Output};

    #[should_panic(
        expected = "expected to generate environment from terrain for biome Some(\"invalid_biome_name\")"
    )]
    #[test]
    fn create_script_will_panic_if_invalid_biome_name() {
        let zsh = Zsh::build(MockRun::default());

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
        let mut mock_wait = MockRun::default();
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

        let mut mock_run = MockRun::default();
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
