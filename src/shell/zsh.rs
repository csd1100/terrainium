use anyhow::{Context, Result};
use mockall_double::double;
use std::{collections::BTreeMap, path::Path, process::Output};

#[cfg(test)]
use mockall::automock;

use crate::{
    helpers::operations::write_file,
    types::biomes::{Biome, BiomeWithName},
};

#[double]
use crate::shell::process::spawn;

const MAIN_TEMPLATE: &str = include_str!("../../templates/zsh_final_script.hbs");
const ALIAS_TEMPLATE: &str = include_str!("../../templates/zsh_aliases.hbs");
const ENV_TEMPLATE: &str = include_str!("../../templates/zsh_env.hbs");
const CONSTRUCTORS_TEMPLATE: &str = include_str!("../../templates/zsh_constructors.hbs");
const DESTRUCTORS_TEMPLATE: &str = include_str!("../../templates/zsh_destructors.hbs");

fn run_via_zsh(args: Vec<&str>, envs: Option<BTreeMap<String, String>>) -> Result<Output> {
    let mut args = args;
    let mut zsh_args = vec!["-c"];
    zsh_args.append(&mut args);

    spawn::and_get_output("/bin/zsh", zsh_args, envs)
}

fn generate_zsh_script(
    central_store: &Path,
    biome_name: &String,
    environment: Biome,
) -> Result<()> {
    let mut handlebar = handlebars::Handlebars::new();
    handlebar.set_strict_mode(true);
    handlebar.register_template_string("main", MAIN_TEMPLATE)?;
    handlebar.register_template_string("envs", ENV_TEMPLATE)?;
    handlebar.register_template_string("aliases", ALIAS_TEMPLATE)?;
    handlebar.register_template_string("constructors", CONSTRUCTORS_TEMPLATE)?;
    handlebar.register_template_string("destructors", DESTRUCTORS_TEMPLATE)?;

    let text = handlebar
        .render(
            "main",
            &BiomeWithName {
                name: biome_name.to_string(),
                biome: environment,
            },
        )
        .context("failed to render script template")?;

    let mut path = central_store.to_path_buf();
    path.push(format!("terrain-{}.zsh", biome_name));

    write_file(&path, text).context(format!("failed to write file to path {:?}", &path))?;
    Ok(())
}

fn compile(central_store: &Path, biome_name: &String) -> Result<()> {
    let mut zsh = central_store.to_path_buf();
    zsh.push(format!("terrain-{}.zsh", biome_name));
    let zsh = zsh.to_string_lossy().to_string();

    let mut zwc = central_store.to_path_buf();
    zwc.push(format!("terrain-{}.zwc", biome_name));
    let zwc = zwc.to_string_lossy().to_string();

    let command = format!("zcompile -URz {} {}", zwc, zsh);
    run_via_zsh(vec![&command], None)?;

    Ok(())
}

fn get_fpath() -> Result<String> {
    let some = "echo -n $FPATH";
    let envs: BTreeMap<String, String> = std::env::vars().collect();
    let output = run_via_zsh(vec![some], Some(envs))?;
    Ok(String::from_utf8(output.stdout)?)
}

#[cfg_attr(test, automock)]
pub mod ops {
    use anyhow::{Context, Result};
    use mockall_double::double;
    use std::{collections::BTreeMap, path::Path};

    use crate::{
        helpers::{
            constants::{FPATH, TERRAINIUM_INIT_FILE, TERRAINIUM_INIT_ZSH},
            operations::get_central_store_path,
            utils::Paths,
        },
        types::biomes::Biome,
    };

    #[double]
    use crate::shell::process::spawn;
    use crate::shell::zsh::{compile, generate_zsh_script, get_fpath};

    pub fn generate_and_compile(
        central_store: &Path,
        biome_name: String,
        environment: Biome,
    ) -> Result<()> {
        generate_zsh_script(central_store, &biome_name, environment).context(format!(
            "failed to generate zsh script for biome {}",
            &biome_name
        ))?;
        compile(central_store, &biome_name).context(format!(
            "failed to compile generated zsh script for biome {}",
            &biome_name
        ))?;
        Ok(())
    }

    pub fn spawn<'a>(args: Vec<&'a str>, envs: Option<BTreeMap<String, String>>) -> Result<()> {
        let mut args = args;
        let mut zsh_args = vec!["-i"];
        zsh_args.append(&mut args);

        spawn::and_wait("/bin/zsh", zsh_args, envs)?;
        Ok(())
    }

    pub fn get_zsh_envs(biome_name: String, paths: &Paths) -> Result<BTreeMap<String, String>> {
        let mut init_file =
            get_central_store_path(paths).context("unable to get terrains config path")?;
        init_file.push(format!("terrain-{}.zwc", &biome_name));
        let init_file = init_file.to_string_lossy().to_string();

        let mut envs = BTreeMap::<String, String>::new();
        envs.insert(TERRAINIUM_INIT_FILE.to_string(), init_file.clone());
        envs.insert(
            TERRAINIUM_INIT_ZSH.to_string(),
            format!("terrain-{}.zsh", biome_name),
        );
        let fpath = format!(
            "{}:{}",
            init_file,
            get_fpath().context("unable to get zsh fpath")?
        );
        envs.insert(FPATH.to_string(), fpath);

        Ok(envs)
    }
}

#[cfg(test)]
mod test {
    use crate::helpers::utils::get_paths;
    use crate::helpers::utils::test_helpers::{
        generate_terrain_central_store_path, generate_terrain_compiled_zwc_path,
    };
    use crate::shell::zsh::ops::{generate_and_compile, get_zsh_envs, spawn};
    use crate::{
        shell::process::mock_spawn,
        types::{args::BiomeArg, terrain::test_data},
    };
    use anyhow::Result;
    use serial_test::serial;
    use std::{
        collections::BTreeMap,
        fs,
        os::unix::process::ExitStatusExt,
        path::PathBuf,
        process::{ExitStatus, Output},
    };
    use tempfile::tempdir;

    #[test]
    #[serial]
    fn generates_and_compiles_all() -> Result<()> {
        let test_dir = tempdir()?;
        let home_dir = tempdir()?;
        let test_dir_path: PathBuf = test_dir.path().into();
        let home_dir_path: PathBuf = home_dir.path().into();

        let paths = get_paths(home_dir_path, test_dir_path)?;

        let central_storage = generate_terrain_central_store_path(&paths)?;
        let terrain = test_data::terrain_full();
        let exp_args = vec![
            "-c".to_owned(),
            "zcompile -URz ".to_owned()
                + &central_storage
                + "/terrain-example_biome.zwc "
                + &central_storage
                + "/terrain-example_biome.zsh",
        ];
        let mock_spawn_get_output = mock_spawn::and_get_output_context();
        mock_spawn_get_output
            .expect()
            .withf(move |exe, args, envs| {
                let exe_eq = exe == "/bin/zsh";
                let args_eq = *args == exp_args;
                let envs_eq = envs.is_none();
                exe_eq && args_eq && envs_eq
            })
            .return_once(|_, _, _| {
                Ok(Output {
                    status: ExitStatus::from_raw(0),
                    stdout: Vec::<u8>::new(),
                    stderr: Vec::<u8>::new(),
                })
            });

        fs::create_dir_all(PathBuf::from(&central_storage)).expect("to be created");

        generate_and_compile(
            &PathBuf::from(central_storage),
            "example_biome".to_string(),
            terrain.get(Some(BiomeArg::Default))?,
        )?;

        let expected = std::fs::read_to_string("./tests/data/terrain-example_biome.zsh")
            .expect("to be present");
        let actual = std::fs::read_to_string(
            generate_terrain_central_store_path(&paths)? + "/terrain-example_biome.zsh",
        )
        .expect("to be present");
        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    #[serial]
    fn spawn_with_args_and_envs() -> Result<()> {
        let exp_args = vec!["-i"];
        let mut exp_envs = BTreeMap::<String, String>::new();
        exp_envs.insert("k".to_string(), "v".to_string());

        let mock_spawn_and_wait = mock_spawn::and_wait_context();
        mock_spawn_and_wait
            .expect()
            .withf(move |exe, args, envs| {
                let exe_eq = exe == "/bin/zsh";
                let args_eq = *args == exp_args;
                let envs_eq = *envs.as_ref().unwrap() == exp_envs;
                exe_eq && args_eq && envs_eq
            })
            .return_once(|_, _, _| Ok(()));

        let mut envs = BTreeMap::<String, String>::new();
        envs.insert("k".to_string(), "v".to_string());
        spawn(vec![], Some(envs))?;

        Ok(())
    }

    #[test]
    #[serial]
    fn get_zsh_env() -> Result<()> {
        let test_dir = tempdir()?;
        let home_dir = tempdir()?;
        let test_dir_path: PathBuf = test_dir.path().into();
        let home_dir_path: PathBuf = home_dir.path().into();

        let paths = get_paths(home_dir_path, test_dir_path)?;

        let exp_args = vec!["-c", "echo -n $FPATH"];
        let mock_spawn_get_output = mock_spawn::and_get_output_context();
        mock_spawn_get_output
            .expect()
            .withf(move |exe, args, envs| {
                let exe_eq = exe == "/bin/zsh";
                let args_eq = *args == exp_args;
                let envs_eq = *envs.as_ref().unwrap() == std::env::vars().collect();

                exe_eq && args_eq && envs_eq
            })
            .return_once(|_, _, _| {
                Ok(Output {
                    status: ExitStatus::from_raw(0),
                    stdout: Vec::<u8>::from("/tmp/test/path"),
                    stderr: Vec::<u8>::new(),
                })
            });

        let mut expected = BTreeMap::<String, String>::new();
        expected.insert(
            "TERRAINIUM_INIT_FILE".to_string(),
            generate_terrain_compiled_zwc_path(&paths)?,
        );
        expected.insert(
            "TERRAINIUM_INIT_ZSH".to_string(),
            "terrain-example_biome.zsh".to_string(),
        );
        expected.insert(
            "FPATH".to_string(),
            generate_terrain_compiled_zwc_path(&paths)? + ":/tmp/test/path",
        );

        let actual = get_zsh_envs("example_biome".to_string(), &paths)?;

        assert_eq!(expected, actual);

        Ok(())
    }

    #[allow(dead_code)]
    const DEFAULT_BIOME_ZSH: &str = "# This file is auto-generated by terrainium
# DO NOT EDIT MANUALLY USE `terrainium edit` COMMAND TO EDIT TOML

function {
    # USER DEFINED ALIASES: START
    alias tedit=\"terrainium edit\"
    alias tenter=\"terrainium enter --biome example_biome\"
    # USER DEFINED ALIASES: END
    # USER DEFINED ENVS: START
    export EDITOR=\"nvim\"
    export TEST=\"value\"
    # USER DEFINED ENVS: END
}

function terrainium_shell_constructor() {
    if [ \"$TERRAINIUM_ENABLED\" = \"true\" ]; then
        echo entering terrain
        echo entering biome 'example_biome'
    fi
}

function terrainium_shell_destructor() {
    if [ \"$TERRAINIUM_ENABLED\" = \"true\" ]; then
        echo exiting terrain
        echo exiting biome 'example_biome'
    fi
}

function terrainium_enter() {
    \"$TERRAINIUM_EXECUTABLE\" construct
    terrainium_shell_constructor
}

function terrainium_exit() {
    if [ \"$TERRAINIUM_ENABLED\" = \"true\" ]; then
        builtin exit
    fi
}

function terrainium_preexec_functions() {
    tenter=\"(\\$TERRAINIUM_EXECUTABLE enter*|$TERRAINIUM_EXECUTABLE enter*|*terrainium enter*)\"
    texit=\"(\\$TERRAINIUM_EXECUTABLE exit*|$TERRAINIUM_EXECUTABLE exit*|*terrainium exit*)\"
    tconstruct=\"(\\$TERRAINIUM_EXECUTABLE construct*|$TERRAINIUM_EXECUTABLE construct*|*terrainium construct*)\"
    tdeconstruct=\"(\\$TERRAINIUM_EXECUTABLE deconstruct*|$TERRAINIUM_EXECUTABLE deconstruct*|*terrainium deconstruct*)\"

    if [ $TERRAINIUM_ENABLED = \"true\" ]; then
        case \"$3\" in
        $~texit)
            terrainium_exit
        ;;
        $~tconstruct)
            terrainium_shell_constructor
        ;;
        $~tdeconstruct)
            terrainium_shell_destructor
        ;;
        esac
    fi
}

function terrainium_chpwd_functions() {
    if [ \"$TERRAINIUM_ENABLED\" != \"true\" ]; then
        if [ \"$TERRAINIUM_AUTO_APPLY\" = 1 ]; then
            \"$TERRAINIUM_EXECUTABLE\" enter
        fi
    fi
}

function terrainium_zshexit_functions() {
    \"$TERRAINIUM_EXECUTABLE\" deconstruct
    terrainium_shell_destructor
}

preexec_functions=(terrainium_preexec_functions $preexec_functions)
chpwd_functions=(terrainium_chpwd_functions $chpwd_functions)
zshexit_functions=(terrainium_zshexit_functions $zshexit_functions)
";
}
