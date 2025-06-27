use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context as AnyhowContext, Result, bail};
use home::home_dir;
use serde::Serialize;
use tracing::warn;

use crate::client::args::BiomeArg;
use crate::client::shell::{Shell, Zsh, render};
use crate::client::types::context::Context;
use crate::client::types::environment::Environment;
use crate::client::types::terrain::{AutoApply, Terrain};
use crate::common::constants::{
    FPATH, NONE, TERRAIN_AUTO_APPLY, TERRAIN_DIR, TERRAIN_INIT_FN, TERRAIN_INIT_SCRIPT,
    TERRAIN_NAME, TERRAIN_SELECTED_BIOME, TERRAIN_SESSION_ID,
};
use crate::common::execute::Execute;
#[mockall_double::double]
use crate::common::execute::Executor;
use crate::common::types::command::Command;

pub const ZSH_INIT_SCRIPT_NAME: &str = "terrainium_init.zsh";

const MAIN_TEMPLATE: &str = include_str!("../../../templates/zsh_final_script.hbs");

#[derive(Serialize)]
struct ScriptData {
    environment: Environment,
    typeset: Vec<&'static str>,
}

fn re_un_exports() -> Vec<&'static str> {
    vec![
        FPATH,
        TERRAIN_NAME,
        TERRAIN_SESSION_ID,
        TERRAIN_SELECTED_BIOME,
        TERRAIN_AUTO_APPLY,
        TERRAIN_DIR,
    ]
}

fn unsets() -> Vec<&'static str> {
    vec![TERRAIN_INIT_SCRIPT, TERRAIN_INIT_FN]
}

fn get_exports(which: char) -> String {
    re_un_exports()
        .into_iter()
        .map(|e| {
            format!(
                "{: <4}if [ -n \"${e}\" ]; then typeset {which}x {e}; fi",
                ""
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn get_unsets() -> String {
    unsets()
        .into_iter()
        .map(|e| format!("{: <4}unset {e}", ""))
        .collect::<Vec<_>>()
        .join("\n")
}

fn get_debug_command_check() -> &'static str {
    if cfg!(debug_assertions) {
        r#"
    elif [ "${command[1]} ${command[2]}" = "cargo run" ] && [ "$TERRAINIUM_DEV" = "true" ]; then
        typeset +x  __terrainium_is_terrain="true"
        typeset +x  __terrainium_verb="${command[4]}""#
    } else {
        ""
    }
}

impl Shell for Zsh {
    fn get(cwd: &Path, executor: Arc<Executor>) -> Self {
        Self {
            bin: "/bin/zsh".to_string(),
            cwd: cwd.to_path_buf(),
            executor,
        }
    }

    fn command(&self) -> Command {
        Command::new(self.bin.to_string(), vec![], Some(self.cwd.clone()))
    }

    fn get_init_rc_contents() -> String {
        format!(
            r#"
source "$HOME/.config/terrainium/shell_integration/{ZSH_INIT_SCRIPT_NAME}"
"#,
        )
    }

    fn get_integration_script(&self) -> String {
        format!(
            r#"#!/usr/bin/env zsh

function __terrainium_auto_apply() {{
    auto_apply="$(terrain get --auto-apply 2> /dev/null)"
    if [ $? != 0 ]; then
        auto_apply="{}"
    fi

    typeset -x FPATH
    if [ "$auto_apply" = "{}" ] || [ "$auto_apply" = "{}" ]; then
        terrain enter --auto-apply
    elif [ "$auto_apply" = "{}" ] || [ "$auto_apply" = "{}" ]; then
        exec terrain enter --auto-apply
    fi
    typeset +x FPATH
}}

function __terrainium_parse_command() {{
    local command=(${{(s/ /)1}})
    if [ "${{command[1]}}" = "terrain" ]; then
        typeset +x __terrainium_is_terrain="true"
        typeset +x __terrainium_verb="${{command[2]}}"{}
    fi
}}

function __terrainium_reexport_envs() {{
{}
    typeset +x __TERRAIN_ENVS_EXPORTED="true"
}}

function __terrainium_unexport_envs() {{
    # unexport but set terrainium env vars
{}
    unset __TERRAIN_ENVS_EXPORTED
}}

function __terrainium_fpath_preexec_function() {{
    __terrainium_parse_command "$3"
    if [ "$__terrainium_is_terrain" = "true" ]; then
        typeset -x FPATH
    fi
}}

function __terrainium_fpath_precmd_function() {{
    if [ "$__terrainium_is_terrain" = "true" ]; then
        typeset +x FPATH
        unset __terrainium_is_terrain
        unset __terrainium_verb
    fi
}}

function __terrainium_chpwd_functions() {{
    __terrainium_auto_apply
}}

if [ -n "$TERRAIN_SESSION_ID" ]; then
    autoload -Uzw "${{TERRAIN_INIT_SCRIPT}}"
    "${{terrain_init}}"
    builtin unfunction -- "${{terrain_init}}"
    __terrainium_enter
    __terrainium_unexport_envs
{}
else
    preexec_functions=(__terrainium_fpath_preexec_function $preexec_functions)
    precmd_functions=(__terrainium_fpath_precmd_function $precmd_functions)
    chpwd_functions=(__terrainium_chpwd_functions $chpwd_functions)
    __terrainium_auto_apply
fi
"#,
            AutoApply::Off,
            AutoApply::Enabled,
            AutoApply::Background,
            AutoApply::Replace,
            AutoApply::All,
            get_debug_command_check(),
            get_exports('-'),
            get_exports('+'),
            get_unsets(),
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
        let script = self.get_integration_script();

        if !fs::exists(&init_script_location)
            .context("failed to check if shell integration script exists")?
        {
            warn!(
                "shell-integration script not found in config directory, copying script to config \
                 directory"
            );

            fs::write(&init_script_location, script)
                .context("failed to create shell-integration script file")?;
        } else if fs::read_to_string(&init_script_location)
            .context("failed to read shell-integration script")?
            != script
        {
            let backup = init_script_location.with_extension("zsh.bkp");

            fs::copy(&init_script_location, backup)
                .context("failed to backup shell-integration script")?;

            fs::remove_file(&init_script_location)
                .context("failed to remove outdated shell-integration script")?;

            warn!(
                "shell-integration script was outdated in config directory, copying newer script \
                 to config directory"
            );

            fs::write(&init_script_location, script)
                .context("failed to create updated shell-integration script file")?;
        }

        let compiled_path = init_script_location.with_extension("zwc");

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
        envs: Option<Arc<BTreeMap<String, String>>>,
    ) -> Result<Output> {
        let mut final_args = vec!["-c".to_string()];
        final_args.append(&mut args);

        let mut command = self.command();
        command.set_args(final_args);

        self.executor
            .get_output(envs, command)
            .context("failed to execute zsh command due to an error")
    }

    async fn spawn(&self, envs: Option<Arc<BTreeMap<String, String>>>) -> Result<ExitStatus> {
        let mut command = self.command();
        command.set_args(vec!["-i".to_string(), "-s".to_string()]);

        self.executor
            .async_spawn(envs, command)
            .await
            .context("failed to run zsh")
    }

    fn generate_envs(&self, scripts_dir: PathBuf, biome: &str) -> Result<BTreeMap<String, String>> {
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
            "export".to_string(),
            r#"{{#if this}}
{{#each this}}
export {{@key}}="{{{this}}}"
{{/each}}
{{/if}}"#
                .to_string(),
        );
        templates.insert(
            "unset".to_string(),
            r#"{{#if this}}
{{#each this}}
    unset {{@key}}
{{/each}}
{{/if}}"#
                .to_string(),
        );
        templates.insert(
            "alias".to_string(),
            r#"{{#if this}}
{{#each this}}
alias {{@key}}="{{{this}}}"
{{/each}}
{{/if}}"#
                .to_string(),
        );
        templates.insert(
            "unalias".to_string(),
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
        if let Ok(fpath) = std::env::var(FPATH) {
            return Ok(fpath);
        }

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
        script_path: &Path,
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

        let script = render(
            "zsh".to_string(),
            Zsh::templates(),
            ScriptData {
                environment,
                typeset: re_un_exports(),
            },
        )
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
    use std::collections::HashSet;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use tempfile::tempdir;

    use crate::client::args::BiomeArg;
    use crate::client::shell::zsh::{re_un_exports, unsets};
    use crate::client::shell::{Shell, Zsh};
    use crate::client::test_utils::assertions::zsh::ExpectZSH;
    use crate::client::test_utils::constants::{
        WITH_EXAMPLE_BIOME_FOR_EXAMPLE_SCRIPT, ZSH_INTEGRATION_SCRIPT,
        ZSH_INTEGRATION_SCRIPT_RELEASE,
    };
    use crate::client::types::environment::Environment;
    use crate::client::types::terrain::Terrain;
    use crate::common::constants::{EXAMPLE_BIOME, FPATH, NONE};
    use crate::common::execute::MockExecutor;

    #[test]
    fn creates_script() {
        let script_dir = tempdir().unwrap();
        let terrain = Terrain::example();

        let script_path = script_dir.path().join("terrain-example_biome.zsh");

        Zsh::get(&PathBuf::new(), Arc::new(MockExecutor::new()))
            .create_script(
                &terrain,
                EXAMPLE_BIOME.to_string(),
                script_path.as_path(),
                &PathBuf::from("/home/user/work/terrainium"),
            )
            .expect("creating script failed");

        let expected =
            fs::read_to_string(Path::new(WITH_EXAMPLE_BIOME_FOR_EXAMPLE_SCRIPT)).unwrap();

        let actual = fs::read_to_string(script_path).unwrap();
        assert_eq!(expected, actual);
    }

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
        fs::write(temp_dir.path().join(".zshrc"), "").unwrap();
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
        fs::create_dir_all(&zsh_integration_script_location).unwrap();

        let zsh_integration_script = zsh_integration_script_location.join("terrainium_init.zsh");
        let compiled_zsh_integration_script = zsh_integration_script.with_extension("zwc");

        let executor = ExpectZSH::with(MockExecutor::new(), home_dir.path())
            .compile_script_successfully_for(
                &zsh_integration_script,
                &compiled_zsh_integration_script,
            )
            .successfully();

        Zsh::get(home_dir.path(), Arc::new(executor))
            .setup_integration(zsh_integration_script_location)
            .expect("to succeed");

        let file_name = if cfg!(debug_assertions) {
            ZSH_INTEGRATION_SCRIPT
        } else {
            ZSH_INTEGRATION_SCRIPT_RELEASE
        };

        let expected = fs::read_to_string(file_name).unwrap();
        let actual = fs::read_to_string(&zsh_integration_script).unwrap();

        assert_eq!(actual, expected);
        assert!(
            fs::exists(zsh_integration_script)
                .expect("failed to check if shell integration script created")
        );
    }

    #[test]
    fn shell_integration_replace() {
        let home_dir = tempdir().unwrap();

        let zsh_integration_script_location =
            home_dir.path().join(".config/terrainium/shell_integration");
        fs::create_dir_all(&zsh_integration_script_location).unwrap();

        let zsh_integration_script = zsh_integration_script_location.join("terrainium_init.zsh");
        let zsh_integration_script_backup = zsh_integration_script.with_extension("zsh.bkp");
        let compiled_zsh_integration_script = zsh_integration_script.with_extension("zwc");

        fs::write(
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

        assert!(
            fs::exists(zsh_integration_script)
                .expect("failed to check if shell integration script created")
        );

        assert!(
            fs::exists(zsh_integration_script_backup)
                .expect("failed to check if shell integration script created")
        );
    }

    #[test]
    fn assert_re_un_exports() {
        // added tests to keep them in sync with actual values
        let environment =
            Environment::from(&Terrain::example(), BiomeArg::None, Path::new("")).unwrap();

        let mut vars = environment
            .activation_env_vars(String::new(), Path::new(""), true)
            .keys()
            .map(ToOwned::to_owned)
            .collect::<HashSet<String>>();
        vars.insert(FPATH.to_owned());

        let actual = re_un_exports()
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<HashSet<String>>();

        assert_eq!(actual, vars);
    }

    #[test]
    fn assert_unsets() {
        // added tests to keep them in sync with actual values
        let executor = ExpectZSH::with(MockExecutor::new(), Path::new(""))
            .get_fpath()
            .successfully();

        let zsh = Zsh::get(Path::new(""), Arc::new(executor));

        let mut vars = zsh
            .generate_envs(PathBuf::new(), NONE)
            .unwrap()
            .keys()
            .map(ToOwned::to_owned)
            .collect::<HashSet<String>>();
        vars.remove(FPATH);

        let actual = unsets()
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<HashSet<String>>();

        assert_eq!(actual, vars);
    }
}
