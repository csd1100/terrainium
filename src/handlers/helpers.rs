use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Ok, Result};
use home::home_dir;

use crate::types::{
    commands::{Command, Commands},
    errors::TerrainiumErrors,
    terrain::{parse_terrain, Terrain},
};

pub fn get_config_path() -> Result<PathBuf> {
    let home = home_dir();
    if let Some(home) = home {
        if Path::is_dir(home.as_path()) {
            let config_dir = Path::join(&home, PathBuf::from(".config"));
            if Path::try_exists(&config_dir.as_path())? {
                return Ok(config_dir);
            }
            return Ok(home);
        } else {
            return Err(TerrainiumErrors::InvalidHomeDirectory.into());
        }
    }
    return Err(TerrainiumErrors::UnableToFindHome.into());
}

fn create_dir_if_not_exist(dir: &Path) -> Result<bool> {
    if !Path::try_exists(dir)? {
        println!("creating a directory at path {}", dir.to_string_lossy());
        std::fs::create_dir_all(dir)?;
        return Ok(true);
    }
    return Ok(false);
}

pub fn get_terrainium_config_path() -> Result<PathBuf> {
    return Ok(Path::join(&get_config_path()?, "terrainium"));
}

pub fn get_central_store_path() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;

    let terrain_dir = Path::canonicalize(cwd.as_path())?
        .to_string_lossy()
        .to_string()
        .replace("/", "_");
    let dirname = Path::join(&get_terrainium_config_path()?, "terrains");
    let dirname = dirname.join(terrain_dir);
    return Ok(dirname);
}

pub fn create_config_dir() -> Result<PathBuf> {
    let config_path = get_terrainium_config_path()?;
    create_dir_if_not_exist(config_path.as_path())
        .context("unable to create terrainium config directory")?;
    create_dir_if_not_exist(get_central_store_path()?.as_path())
        .context("unable to create terrains directory in terrainium config directory")?;
    return Ok(config_path);
}

pub fn get_central_terrain_path() -> Result<PathBuf> {
    let mut dirname = get_central_store_path()?;
    dirname.push("terrain.toml");
    return Ok(dirname);
}

pub fn get_local_terrain_path() -> Result<PathBuf> {
    return Ok(Path::join(&std::env::current_dir()?, "terrain.toml"));
}

pub fn is_local_terrain_present() -> Result<bool> {
    return Ok(Path::try_exists(&get_local_terrain_path()?)?);
}

pub fn is_central_terrain_present() -> Result<bool> {
    return Ok(Path::try_exists(&get_central_terrain_path()?)?);
}

pub fn get_terrain_toml() -> Result<PathBuf> {
    if is_local_terrain_present()? {
        return get_local_terrain_path();
    } else if is_central_terrain_present()? {
        return get_central_terrain_path();
    } else {
        let err = anyhow!("unable to get terrain.toml for this project. initialize terrain with `terrainium init` command");
        return Err(err);
    }
}

pub fn get_parsed_terrain() -> Result<Terrain> {
    let toml_file = get_terrain_toml().context("unable to get terrain.toml path")?;
    return parse_terrain(&toml_file);
}

pub fn merge_hashmaps(
    to: &HashMap<String, String>,
    from: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut return_map = to.clone();
    let _: Vec<_> = from
        .iter()
        .map(|(key, value)| return_map.insert(key.to_string(), value.to_string()))
        .collect();
    return return_map;
}

pub fn get_merged_hashmaps(
    from: &Option<HashMap<String, String>>,
    to: &Option<HashMap<String, String>>,
) -> Option<HashMap<String, String>> {
    if from.is_some() && !to.is_some() {
        return from.clone();
    }
    if to.is_some() && !from.is_some() {
        return to.clone();
    }

    if let Some(from) = &from {
        if let Some(to) = &to {
            return Some(merge_hashmaps(from, to));
        }
    }

    return None;
}

pub fn get_merged_vecs(
    from: &Option<Vec<Command>>,
    to: &Option<Vec<Command>>,
) -> Option<Vec<Command>> {
    if from.is_none() && to.is_none() {
        return None;
    }
    if from.is_some() && !to.is_some() {
        return from.clone();
    }
    if to.is_some() && !from.is_some() {
        return to.clone();
    }

    let mut return_vec = to.clone().expect("to be present");
    return_vec.extend_from_slice(&from.clone().expect("to be present"));
    return Some(return_vec);
}

pub fn get_merged_commands(from: &Option<Commands>, to: &Option<Commands>) -> Option<Commands> {
    if from.is_some() && !to.is_some() {
        return from.clone();
    }
    if to.is_some() && !from.is_some() {
        return to.clone();
    }

    if let Some(from) = &from {
        if let Some(to) = &to {
            return Some(to.merge(from.clone()));
        }
    }

    return None;
}

pub fn find_in_hashmaps(
    from: &Option<HashMap<String, String>>,
    tofind: Vec<String>,
) -> Result<HashMap<String, Option<String>>> {
    let return_map: HashMap<String, Option<String>> = if let Some(values) = from {
        tofind
            .iter()
            .map(|env| {
                if let Some(value) = values.get(env) {
                    return (env.to_string(), Some(value.to_string()));
                } else {
                    return (env.to_string(), None);
                }
            })
            .collect()
    } else {
        return Err(anyhow!("Not Defined"));
    };
    return Ok(return_map);
}

pub fn get_process_log_file_path(session_id: &String, filename: String) -> Result<PathBuf> {
    let tmp = PathBuf::from(format!("/tmp/terrainium-{}", session_id));
    create_dir_if_not_exist(&tmp)?;
    let mut out_path = tmp.clone();
    out_path.push(filename);
    return Ok(out_path);
}
