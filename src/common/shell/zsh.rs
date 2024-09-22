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
use anyhow::Result;
use mockall_double::double;
use std::collections::BTreeMap;
use std::fs;
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

        terrain.biomes().keys().for_each(|biome_name| {
            let mut script_path = scripts_dir.clone();
            script_path.set_file_name(format!("terrain-{}.zsh", biome_name));
            create_script(&terrain, Some(biome_name.to_string()), &script_path);

            let mut compiled_script_path = scripts_dir.clone();
            compiled_script_path.set_file_name(format!("terrain-{}.zwc", biome_name));
            self.compile_script(&script_path, &compiled_script_path);
        });
        let mut script_path = scripts_dir.clone();
        script_path.set_file_name("terrain-none.zsh");
        create_script(&terrain, Some("none".to_string()), &script_path);

        let mut compiled_script_path = scripts_dir.clone();
        compiled_script_path.set_file_name("terrain-none.zwc");
        self.compile_script(&script_path, &compiled_script_path);

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

fn create_script(terrain: &Terrain, biome_name: Option<String>, script_path: &PathBuf) {
    let environment =
        Environment::from(terrain, biome_name).expect("failed to get environment from terrain");
    let script = environment
        .to_rendered(ZSH_MAIN_TEMPLATE_NAME.to_string(), Zsh::templates())
        .expect("scripts to be rendered");

    fs::write(script_path, script).expect("file to be written");
}

impl Zsh {
    fn compile_script(&self, script_path: &Path, compiled_script_path: &Path) {
        let args = vec![format!(
            "zcompile -URz {} {}",
            compiled_script_path.to_string_lossy(),
            script_path.to_string_lossy()
        )];

        self.execute(args, None).expect("to succeed");
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
