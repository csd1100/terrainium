use std::{collections::HashMap, path::PathBuf, process::Output};

use anyhow::Result;

use crate::{
    handlers::{
        constants::{FPATH, TERRAINIUM_INIT_FILE, TERRAINIUM_INIT_ZSH},
        helpers::get_central_store_path,
    },
    shell::execute::spawn_and_wait,
    types::biomes::Biome,
};

use super::execute::run_and_get_output;

const MAIN_TEMPLATE: &str = include_str!("../../templates/zsh_final_script.hbs");
const ALIAS_TEMPLATE: &str = include_str!("../../templates/zsh_aliases.hbs");
const ENV_TEMPLATE: &str = include_str!("../../templates/zsh_env.hbs");
const CONSTRUCTORS_TEMPLATE: &str = include_str!("../../templates/zsh_constructors.hbs");
const DESTRUCTORS_TEMPLATE: &str = include_str!("../../templates/zsh_destructors.hbs");
pub const ZSH_FILE: &str = "terrain.zsh";
pub const ZWC_FILE: &str = "terrain.zwc";

pub fn generate_zsh_script(path: &PathBuf, terrain: Biome) -> Result<()> {
    let mut handlebar = handlebars::Handlebars::new();
    handlebar.set_strict_mode(true);
    handlebar.register_template_string("main", MAIN_TEMPLATE)?;
    handlebar.register_template_string("envs", ENV_TEMPLATE)?;
    handlebar.register_template_string("aliases", ALIAS_TEMPLATE)?;
    handlebar.register_template_string("constructors", CONSTRUCTORS_TEMPLATE)?;
    handlebar.register_template_string("destructors", DESTRUCTORS_TEMPLATE)?;

    let text = handlebar.render("main", &terrain)?;
    let mut path = path.clone();
    path.push(ZSH_FILE);
    println!("updating environment scripts");
    std::fs::write(path, &text)?;
    return Ok(());
}

pub fn run_via_zsh(args: Vec<&str>, envs: Option<HashMap<String, String>>) -> Result<Output> {
    let mut args = args;
    let mut zsh_args = vec!["-c"];
    zsh_args.append(&mut args);

    return run_and_get_output("/bin/zsh", zsh_args, envs);
}

pub fn spawn_zsh(args: Vec<&str>, envs: Option<HashMap<String, String>>) -> Result<()> {
    let mut args = args;
    let mut zsh_args = vec!["-i"];
    zsh_args.append(&mut args);

    spawn_and_wait("/bin/zsh", zsh_args, envs)?;
    return Ok(());
}

pub fn compile(path: &PathBuf) -> Result<()> {
    let mut zsh = path.clone();
    zsh.push(ZSH_FILE);
    let zsh = zsh.to_string_lossy().to_string();

    let mut zwc = path.clone();
    zwc.push(ZWC_FILE);
    let zwc = zwc.to_string_lossy().to_string();

    let command = format!("zcompile -URz {} {}", zwc, zsh);
    println!("[command: {:?}]\n", command);

    println!("compiling zsh scripts");
    run_via_zsh(vec![&command], None)?;

    return Ok(());
}

fn get_fpath() -> Result<String> {
    let some = "echo -n $FPATH";
    let envs: HashMap<String, String> =
        std::env::vars().into_iter().map(|env| return env).collect();
    let output = run_via_zsh(vec![some], Some(envs))?;
    return Ok(String::from_utf8(output.stdout)?);
}

pub fn get_zsh_envs() -> Result<HashMap<String, String>> {
    let mut init_file = get_central_store_path()?;
    init_file.push(ZWC_FILE);
    let init_file = init_file.to_string_lossy().to_string();
    let mut envs = HashMap::<String, String>::new();
    envs.insert(TERRAINIUM_INIT_FILE.to_string(), init_file.clone());
    envs.insert(TERRAINIUM_INIT_ZSH.to_string(), ZSH_FILE.to_string());
    let fpath = format!("{}:{}", init_file, get_fpath()?);
    envs.insert(FPATH.to_string(), fpath);

    return Ok(envs);
}
