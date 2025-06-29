use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::os::unix::prelude::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::Arc;

use anyhow::{bail, Context as _, Result};
use terrainium_lib::command::Command;
use terrainium_lib::executor::Execute;
use tracing::info;

use crate::constants::{
    FPATH, TERRAIN_AUTO_APPLY, TERRAIN_DIR, TERRAIN_INIT_FN, TERRAIN_INIT_SCRIPT, TERRAIN_NAME,
    TERRAIN_SELECTED_BIOME, TERRAIN_SESSION_ID, ZSHRC,
};
use crate::context::Context;
use crate::shell::{Shell, Zsh};
use crate::types::terrain::AutoApply;

const INIT_SCRIPT_NAME: &str = "terrainium_init.zsh";

/// list of environment variables to export only for terrain commands
fn reexports() -> Vec<&'static str> {
    vec![
        FPATH,
        TERRAIN_NAME,
        TERRAIN_SESSION_ID,
        TERRAIN_SELECTED_BIOME,
        TERRAIN_AUTO_APPLY,
        TERRAIN_DIR,
    ]
}

/// list of environment variables to unset after terrain has initialized
fn unsets() -> Vec<&'static str> {
    vec![TERRAIN_INIT_SCRIPT, TERRAIN_INIT_FN]
}

/// typeset commands for all reexports
/// used in shell-integration script
fn typesets(which: char) -> String {
    reexports()
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

/// unset command for all environment variables that will be unset
/// used in shell-integration script
fn get_unsets() -> String {
    unsets()
        .into_iter()
        .map(|e| format!("{: <4}unset {e}", ""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// during development `terrain` command won't be run, rather cargo run will be
/// executed by developers.
/// terrainium environment variables should also be exported during development,
/// so to export required environment variables when cargo run is executed,
/// add a condition to integration script.
fn debug_condition() -> &'static str {
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
    /// [Command] to execute commands using shell
    fn command(&self) -> Command {
        Command::new(self.bin.to_string(), vec![], Some(self.cwd.clone()))
    }

    /// get contents to be added to the rc file to enable shell-integration
    fn get_init_rc_contents(&self) -> String {
        format!(
            r#"
source "$HOME/.config/terrainium/shell_integration/{INIT_SCRIPT_NAME}"
"#,
        )
    }

    /// generate shell-integration script contents
    fn generate_integration_script(&self) -> String {
        self.integration_script()
    }

    /// creates shell-integrations script in `~/.config/terrainium/shell_integration`
    fn create_integration_script(&self, integration_dir: PathBuf) -> Result<()> {
        if !integration_dir.exists() {
            fs::create_dir_all(&integration_dir)
                .context("failed to create shell-integration scripts directory")?;
        }

        let script_path = integration_dir.join(INIT_SCRIPT_NAME);
        let generated_script = self.generate_integration_script();

        if script_path.exists() {
            let script = fs::read_to_string(&script_path)
                .context("failed to read shell-integration script")?;

            if script == generated_script {
                return Ok(());
            }

            info!("shell-integration script was outdated updating it...");
            let backup = script_path.with_extension("zsh.bkp");
            fs::copy(&script_path, backup).context("failed to backup shell-integration script")?;
        }

        fs::write(&script_path, generated_script)
            .context("failed to create shell-integration script file")?;

        let compiled_path = script_path.with_extension("zwc");
        self.compile_script(&script_path, &compiled_path)
    }

    /// get default rc path i.e. `~/.zshrc`.
    fn get_default_rc(&self, home_dir: &Path) -> PathBuf {
        home_dir.join(ZSHRC)
    }

    /// update `~/.zshrc` or specified path to set up shell-integration.
    fn update_rc(&self, home_dir: &Path, path: PathBuf) -> Result<()> {
        self.create_integration_script(Context::shell_integration_dir(home_dir))?;
        let path = fs::canonicalize(path).context("failed to normalize the rc path")?;
        let rc = fs::read_to_string(&path).context("failed to read rc")?;

        if !rc.contains(&self.get_init_rc_contents()) {
            let mut rc_file = fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .context("failed to open rc")?;
            rc_file
                .write_all(self.get_init_rc_contents().as_bytes())
                .context("failed to write rc")?;
        }

        Ok(())
    }

    /// execute [Command]s in shell
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
}

impl Zsh {
    /// creates new instance of zsh shell
    pub(crate) fn get(bin: String, cwd: &Path, executor: Arc<dyn Execute>) -> Self {
        Self {
            bin,
            cwd: cwd.to_path_buf(),
            executor,
        }
    }

    /// compile script using `zcompile` command.
    ///
    /// useful for faster load time of shell
    fn compile_script(&self, script_path: &Path, compiled_script_path: &Path) -> Result<()> {
        let args = vec![format!(
            "zcompile -URz {} {}",
            compiled_script_path.to_string_lossy(),
            script_path.to_string_lossy()
        )];

        let output = self
            .execute(args, None)
            .context(format!("failed to compile script {script_path:?}"))?;

        if output.status.into_raw() != 0 {
            bail!(
                "compiling script failed with exit code {:?}\n error: {}",
                output.status.code(),
                std::str::from_utf8(&output.stderr).expect("failed to convert STDERR to string")
            );
        }

        Ok(())
    }

    /// create integration script contents
    fn integration_script(&self) -> String {
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
            debug_condition(),
            typesets('-'),
            typesets('+'),
            get_unsets(),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use anyhow::Result;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    use super::*;
    use crate::test_helpers::test_zsh::{
        ExpectZSH, ZSH_INTEGRATION_SCRIPT, ZSH_INTEGRATION_SCRIPT_RELEASE,
    };
    use crate::types::terrain::{BiomeArg, Terrain};

    /// zsh binary
    fn bin() -> String {
        "/bin/zsh".to_string()
    }

    #[test]
    fn update_rc_path() -> Result<()> {
        let home_dir = tempdir()?;
        fs::write(home_dir.path().join(".zshrc"), "")?;

        let integration_dir = home_dir.path().join(".config/terrainium/shell_integration");

        let integration_script = integration_dir.join("terrainium_init.zsh");
        let compiled_script = integration_script.with_extension("zwc");

        let executor = ExpectZSH::to(home_dir.path())
            .compile_script_successfully_for(&integration_script, &compiled_script);

        Zsh::get(bin(), home_dir.path(), Arc::new(executor))
            .update_rc(home_dir.path(), home_dir.path().join(ZSHRC))?;

        let expected =
            "\nsource \"$HOME/.config/terrainium/shell_integration/terrainium_init.zsh\"\n";

        assert_eq!(expected, fs::read_to_string(home_dir.path().join(ZSHRC))?);

        Ok(())
    }

    #[test]
    fn shell_integration() -> Result<()> {
        let home_dir = tempdir()?;

        let integration_dir = home_dir.path().join(".config/terrainium/shell_integration");
        let integration_script = integration_dir.join("terrainium_init.zsh");
        let compiled_script = integration_script.with_extension("zwc");

        let executor = ExpectZSH::to(home_dir.path())
            .compile_script_successfully_for(&integration_script, &compiled_script);

        Zsh::get(bin(), home_dir.path(), Arc::new(executor))
            .create_integration_script(integration_dir)
            .expect("to succeed");

        let file_name = if cfg!(debug_assertions) {
            ZSH_INTEGRATION_SCRIPT
        } else {
            ZSH_INTEGRATION_SCRIPT_RELEASE
        };

        let expected = fs::read_to_string(file_name)?;
        let actual = fs::read_to_string(&integration_script)?;

        assert!(integration_script.exists());
        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn shell_integration_replace() -> Result<()> {
        let home_dir = tempdir()?;

        let integration_dir = home_dir.path().join(".config/terrainium/shell_integration");
        fs::create_dir_all(&integration_dir)?;

        let integration_script = integration_dir.join("terrainium_init.zsh");
        let integration_script_bkp = integration_script.with_extension("zsh.bkp");
        let compiled_script = integration_script.with_extension("zwc");

        fs::write(&integration_script, "")?;

        let executor = ExpectZSH::to(home_dir.path())
            .compile_script_successfully_for(&integration_script, &compiled_script);

        Zsh::get(bin(), home_dir.path(), Arc::new(executor))
            .create_integration_script(integration_dir)
            .expect("to succeed");

        assert!(integration_script.exists());
        assert!(integration_script_bkp.exists());

        Ok(())
    }

    #[test]
    fn assert_reexports() -> Result<()> {
        // added tests to keep them in sync with actual values
        let environment = Terrain::example().into_environment(BiomeArg::None)?;

        let mut vars = environment
            .terrainium_vars(String::new(), Path::new(""), true)
            .keys()
            .map(ToOwned::to_owned)
            .collect::<HashSet<String>>();
        vars.insert(FPATH.to_owned());

        let actual = reexports()
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<HashSet<String>>();

        assert_eq!(actual, vars);

        Ok(())
    }

    #[test]
    fn assert_unsets() {
        // added tests to keep them in sync with actual values
        // let executor = ExpectZSH::to(Path::new("")).get_fpath().successfully();
        //
        // let zsh = Zsh::get(Path::new(""), Arc::new(executor));
        //
        // let mut vars = zsh
        //     .generate_envs(PathBuf::new(), NONE)
        //     .unwrap()
        //     .keys()
        //     .map(ToOwned::to_owned)
        //     .collect::<HashSet<String>>();
        // vars.remove(FPATH);
        //
        // let actual = unsets()
        //     .into_iter()
        //     .map(ToOwned::to_owned)
        //     .collect::<HashSet<String>>();
        //
        // assert_eq!(actual, vars);
    }
}
