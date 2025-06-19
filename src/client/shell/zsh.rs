use crate::client::args::BiomeArg;
use crate::client::shell::{Shell, Zsh};
use crate::client::types::context::Context;
use crate::client::types::environment::Environment;
use crate::client::types::terrain::Terrain;
use crate::common::constants::{FPATH, NONE, TERRAIN_INIT_FN, TERRAIN_INIT_SCRIPT};
use crate::common::execute::Execute;
#[mockall_double::double]
use crate::common::execute::Executor;
use crate::common::types::command::Command;
use anyhow::{bail, Context as AnyhowContext, Result};
use home::home_dir;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};
use std::str::FromStr;
use std::sync::Arc;
use tracing::warn;

pub const ZSH_INIT_SCRIPT_NAME: &str = "terrainium_init.zsh";

const INIT_SCRIPT: &str = include_str!("../../scripts/terrainium_init.zsh");
const MAIN_TEMPLATE: &str = include_str!("../../../templates/zsh_final_script.hbs");

impl Shell for Zsh {
    fn get(cwd: &Path, executor: Arc<Executor>) -> Self {
        Self {
            bin: "/bin/zsh".to_string(),
            cwd: cwd.to_path_buf(),
            executor,
        }
    }

    fn command(&self) -> Command {
        Command::new(self.bin.to_string(), vec![], None, Some(self.cwd.clone()))
    }

    fn get_init_rc_contents() -> String {
        format!(
            r#"
source "$HOME/.config/terrainium/shell_integration/{ZSH_INIT_SCRIPT_NAME}"
"#,
        )
    }

    fn setup_integration(&self, integration_scripts_dir: PathBuf) -> Result<()> {
        if !fs::exists(&integration_scripts_dir)
            .context("failed to check if config and shell integration scripts directory exists")?
        {
            fs::create_dir_all(&integration_scripts_dir)
                .context("failed to create shell integration scripts directory")?;
        }

        let init_script_location = integration_scripts_dir.join(ZSH_INIT_SCRIPT_NAME);

        if !fs::exists(&init_script_location)
            .context("failed to check if shell integration script exists")?
        {
            warn!("shell-integration script not found in config directory, copying script to config directory");

            fs::write(&init_script_location, INIT_SCRIPT)
                .context("failed to create shell-integration script file")?;
        } else if fs::read_to_string(&init_script_location)
            .context("failed to read shell-integration script")?
            != INIT_SCRIPT
        {
            let backup = init_script_location.with_extension("zsh.bkp");

            fs::copy(&init_script_location, backup)
                .context("failed to backup shell-integration script")?;

            fs::remove_file(&init_script_location)
                .context("failed to remove outdated shell-integration script")?;

            warn!("shell-integration script was outdated in config directory, copying newer script to config directory");

            fs::write(&init_script_location, INIT_SCRIPT)
                .context("failed to create updated shell-integration script file")?;
        }

        let compiled_path = init_script_location.with_extension("zsh.zwc");

        self.compile_script(&init_script_location, &compiled_path)
    }

    fn update_rc(path: Option<PathBuf>) -> Result<()> {
        let path = path.unwrap_or_else(|| home_dir().expect("cannot get home dir").join(".zshrc"));
        let rc = fs::read_to_string(&path).context("failed to read rc")?;
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
                self.create_and_compile(
                    &terrain,
                    &scripts_dir,
                    biome_name.to_string(),
                    context.terrain_dir(),
                )
                .context(format!("failed to generate scripts for '{biome_name}'"))?;
                Ok(())
            })
            .collect();
        result?;

        self.create_and_compile(
            &terrain,
            &scripts_dir,
            NONE.to_string(),
            context.terrain_dir(),
        )
        .context("failed to generate scripts for 'none'".to_string())?;

        Ok(())
    }

    fn execute(
        &self,
        mut args: Vec<String>,
        envs: Option<BTreeMap<String, String>>,
    ) -> Result<Output> {
        let mut final_args = vec!["-c".to_string()];
        final_args.append(&mut args);

        let mut command = self.command();
        command.set_args(final_args);
        command.set_envs(envs);

        self.executor.get_output(command)
    }

    async fn spawn(&self, envs: BTreeMap<String, String>) -> Result<ExitStatus> {
        let mut command = self.command();
        command.set_args(vec!["-i".to_string(), "-s".to_string()]);
        command.set_envs(Some(envs));

        self.executor
            .async_spawn(command)
            .await
            .context("failed to run zsh")
    }

    fn generate_envs(&self, context: &Context, biome: &str) -> Result<BTreeMap<String, String>> {
        let scripts_dir = context.scripts_dir();
        let compiled_script = Self::compiled_script_path(&scripts_dir, biome)
            .to_str()
            .expect("path to be converted to string")
            .to_string();

        let mut envs = BTreeMap::new();

        let updated_fpath = format!("{compiled_script}:{}", self.get_fpath()?);
        envs.insert(FPATH.to_string(), updated_fpath);
        envs.insert(TERRAIN_INIT_SCRIPT.to_string(), compiled_script);
        envs.insert(TERRAIN_INIT_FN.to_string(), format!("terrain-{biome}.zsh"));

        Ok(envs)
    }

    fn templates() -> BTreeMap<String, String> {
        let mut templates: BTreeMap<String, String> = BTreeMap::new();
        templates.insert("zsh".to_string(), MAIN_TEMPLATE.to_string());
        templates.insert(
            "envs".to_string(),
            r#"{{#if this}}
{{#each this}}
export {{@key}}="{{{this}}}"
{{/each}}
{{/if}}"#
                .to_string(),
        );
        templates.insert(
            "unenvs".to_string(),
            r#"{{#if this}}
{{#each this}}
    unset {{@key}}
{{/each}}
{{/if}}"#
                .to_string(),
        );
        templates.insert(
            "aliases".to_string(),
            r#"{{#if this}}
{{#each this}}
alias {{@key}}="{{{this}}}"
{{/each}}
{{/if}}"#
                .to_string(),
        );
        templates.insert(
            "unaliases".to_string(),
            r#"{{#if this}}
{{#each this}}
    unalias {{@key}}
{{/each}}
{{/if}}"#
                .to_string(),
        );

        templates.insert(
            "commands".to_string(),
            r#"{{#if this}}
{{#if this.foreground}}
{{#each this.foreground}}
    {{#if this}}
        {{#if this.cwd}}
        if pushd {{this.cwd}} &> /dev/null; then
        {{/if}}
            {{this.exe}} {{#each this.args}}{{{this}}}{{/each}}
        {{#if this.cwd}}
            popd &> /dev/null
        fi
        {{/if}}

    {{/if}}
{{/each}}
{{/if}}
{{/if}}"#
                .to_string(),
        );
        templates
    }
}

impl Zsh {
    fn get_fpath(&self) -> Result<String> {
        let cmd = "/bin/echo -n $FPATH";

        let output = self.execute(vec![cmd.to_string()], None)?;

        if !output.status.success() {
            bail!("getting fpath failed with status {}", output.status);
        }
        String::from_utf8(output.stdout).context("failed to convert stdout to string")
    }

    fn create_and_compile(
        &self,
        terrain: &Terrain,
        scripts_dir: &Path,
        biome_name: String,
        terrain_dir: &Path,
    ) -> Result<()> {
        let script_path = Self::script_path(scripts_dir, &biome_name);
        self.create_script(terrain, biome_name.to_string(), &script_path, terrain_dir)?;

        let compiled_script_path = Self::compiled_script_path(scripts_dir, &biome_name);
        self.compile_script(&script_path, &compiled_script_path)?;

        Ok(())
    }

    fn compiled_script_path(scripts_dir: &Path, biome_name: &str) -> PathBuf {
        scripts_dir.join(format!("terrain-{biome_name}.zwc"))
    }

    fn script_path(scripts_dir: &Path, biome_name: &str) -> PathBuf {
        scripts_dir.join(format!("terrain-{biome_name}.zsh"))
    }

    fn create_script(
        &self,
        terrain: &Terrain,
        biome_name: String,
        script_path: &PathBuf,
        terrain_dir: &Path,
    ) -> Result<()> {
        let environment = Environment::from(
            terrain,
            BiomeArg::from_str(&biome_name).unwrap(),
            terrain_dir,
        )
        .context(format!(
            "expected to generate environment from terrain for biome {:?}",
            biome_name
        ))?;

        let script = environment
            .to_rendered("zsh".to_string(), Zsh::templates())
            .context(format!(
                "failed to render script for biome: '{:?}'",
                biome_name
            ))?;

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
            bail!(
                "compiling script failed with exit code {:?}\n error: {}",
                output.status.code(),
                std::str::from_utf8(&output.stderr).expect("failed to convert STDERR to string")
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::client::shell::{Shell, Zsh};
    use crate::client::test_utils::assertions::zsh::ExpectZSH;
    use crate::client::types::terrain::Terrain;
    use crate::common::execute::MockExecutor;
    use std::fs;
    use std::fs::{create_dir_all, exists, write};
    use std::path::PathBuf;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[should_panic(
        expected = "expected to generate environment from terrain for biome \"invalid_biome_name\""
    )]
    #[test]
    fn create_script_will_panic_if_invalid_biome_name() {
        let terrain = Terrain::example();

        Zsh::get(&PathBuf::new(), Arc::new(MockExecutor::new()))
            .create_script(
                &terrain,
                "invalid_biome_name".to_string(),
                &PathBuf::new(),
                &PathBuf::new(),
            )
            .expect("error to be thrown");
    }

    #[test]
    fn compile_script_should_return_error_if_non_zero_exit_code() {
        let path_new = PathBuf::new();
        let path_new = path_new.as_path();

        let executor = ExpectZSH::with(MockExecutor::new(), path_new)
            .compile_script_with_non_zero_exit_code(path_new, path_new)
            .successfully();

        let err = Zsh::get(path_new, Arc::new(executor))
            .compile_script(path_new, path_new)
            .expect_err("error to be thrown");

        assert_eq!(
            "compiling script failed with exit code Some(1)\n error: some error while compiling",
            err.to_string()
        );
    }

    #[test]
    fn update_rc_path() {
        let temp_dir = tempdir().unwrap();
        write(temp_dir.path().join(".zshrc"), "").unwrap();
        Zsh::update_rc(Some(temp_dir.path().join(".zshrc"))).unwrap();

        let expected =
            "\nsource \"$HOME/.config/terrainium/shell_integration/terrainium_init.zsh\"\n";
        assert_eq!(
            expected,
            fs::read_to_string(temp_dir.path().join(".zshrc")).unwrap()
        );
    }

    #[test]
    fn shell_integration() {
        let home_dir = tempdir().unwrap();

        let zsh_integration_script_location =
            home_dir.path().join(".config/terrainium/shell_integration");
        create_dir_all(&zsh_integration_script_location).unwrap();

        let zsh_integration_script = zsh_integration_script_location.join("terrainium_init.zsh");
        let compiled_zsh_integration_script = zsh_integration_script.with_extension("zsh.zwc");

        let executor = ExpectZSH::with(MockExecutor::new(), home_dir.path())
            .compile_script_successfully_for(
                &zsh_integration_script,
                &compiled_zsh_integration_script,
            )
            .successfully();

        Zsh::get(home_dir.path(), Arc::new(executor))
            .setup_integration(zsh_integration_script_location)
            .expect("to succeed");

        assert!(exists(zsh_integration_script)
            .expect("failed to check if shell integration script created"));
    }

    #[test]
    fn shell_integration_replace() {
        let home_dir = tempdir().unwrap();

        let zsh_integration_script_location =
            home_dir.path().join(".config/terrainium/shell_integration");
        create_dir_all(&zsh_integration_script_location).unwrap();

        let zsh_integration_script = zsh_integration_script_location.join("terrainium_init.zsh");
        let zsh_integration_script_backup = zsh_integration_script.with_extension("zsh.bkp");
        let compiled_zsh_integration_script = zsh_integration_script.with_extension("zsh.zwc");

        write(
            home_dir
                .path()
                .join(".config/terrainium/shell_integration/terrainium_init.zsh"),
            "",
        )
        .expect("test shell integration to be written");

        let executor = ExpectZSH::with(MockExecutor::new(), home_dir.path())
            .compile_script_successfully_for(
                &zsh_integration_script,
                &compiled_zsh_integration_script,
            )
            .successfully();

        Zsh::get(home_dir.path(), Arc::new(executor))
            .setup_integration(zsh_integration_script_location)
            .expect("to succeed");

        assert!(exists(zsh_integration_script)
            .expect("failed to check if shell integration script created"));

        assert!(exists(zsh_integration_script_backup)
            .expect("failed to check if shell integration script created"));
    }
}
