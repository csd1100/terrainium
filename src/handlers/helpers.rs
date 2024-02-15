use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Ok, Result};
use home::home_dir;

#[cfg(test)]
use mockall::automock;

use crate::types::{
    errors::TerrainiumErrors,
    terrain::{parse_terrain, Terrain},
};

#[cfg_attr(test, automock)]
pub mod fs {
    use std::path::{Path, PathBuf};

    use anyhow::{Context, Ok, Result};

    use crate::handlers::helpers::get_terrainium_config_path;

    pub fn create_config_dir() -> Result<PathBuf> {
        let config_path =
            get_terrainium_config_path().context("unable to get terrainium config path")?;
        println!("[config_path: {:?}]\n", config_path);
        super::create_dir_if_not_exist(config_path.as_path())
            .context("unable to create terrainium config directory")?;
        super::create_dir_if_not_exist(get_central_store_path()?.as_path())
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

    pub fn is_terrain_present() -> Result<bool> {
        if super::is_local_terrain_present()? {
            return Ok(true);
        } else if super::is_central_terrain_present()? {
            return Ok(true);
        }
        return Ok(false);
    }

    pub fn write_file(path: &Path, contents: String) -> Result<()> {
        return Ok(std::fs::write(path, contents)?);
    }

    pub fn get_central_store_path() -> Result<PathBuf> {
        let cwd = std::env::current_dir()?;

        let terrain_dir = Path::canonicalize(cwd.as_path())?
            .to_string_lossy()
            .to_string()
            .replace("/", "_");
        let dirname = Path::join(
            &get_terrainium_config_path().context("unable to get terrainium config directory")?,
            "terrains",
        );
        let dirname = dirname.join(terrain_dir);
        return Ok(dirname);
    }
}

fn create_dir_if_not_exist(dir: &Path) -> Result<bool> {
    if !Path::try_exists(dir)? {
        println!("creating a directory at path {}", dir.to_string_lossy());
        std::fs::create_dir_all(dir)?;
        return Ok(true);
    }
    return Ok(false);
}

fn get_terrainium_config_path() -> Result<PathBuf> {
    return Ok(Path::join(
        &get_config_path().context("unable to get config directory")?,
        "terrainium",
    ));
}

fn get_config_path() -> Result<PathBuf> {
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

pub fn get_parsed_terrain() -> Result<Terrain> {
    let toml_file = get_terrain_toml().context("unable to get terrain.toml path")?;
    return parse_terrain(&toml_file);
}

pub fn get_terrain_toml() -> Result<PathBuf> {
    if is_local_terrain_present().context("failed to check whether local terrain.toml exists")? {
        return fs::get_local_terrain_path();
    } else if is_central_terrain_present()
        .context("failed to check whether central terrain.toml exists")?
    {
        return fs::get_central_terrain_path();
    } else {
        let err = anyhow!("unable to get terrain.toml for this project. initialize terrain with `terrainium init` command");
        return Err(err);
    }
}

fn is_local_terrain_present() -> Result<bool> {
    return Ok(Path::try_exists(&fs::get_local_terrain_path()?)?);
}

fn is_central_terrain_present() -> Result<bool> {
    return Ok(Path::try_exists(&fs::get_central_terrain_path()?)?);
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

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use anyhow::Result;

    use super::get_merged_hashmaps;

    #[test]
    fn test_merge_hashmaps() -> Result<()> {
        let mut hashmap_1: HashMap<String, String> = HashMap::<String, String>::new();
        hashmap_1.insert("k1".to_string(), "v1".to_string());

        // from: some to: none; from
        let actual = get_merged_hashmaps(&Some(hashmap_1.clone()), &None).expect("to be present");
        assert_eq!(hashmap_1, actual);

        // from: none to: some; to
        let actual = get_merged_hashmaps(&None, &Some(hashmap_1.clone())).expect("to be present");
        assert_eq!(hashmap_1, actual);

        let mut hashmap_2: HashMap<String, String> = HashMap::<String, String>::new();
        hashmap_2.insert("k2".to_string(), "v2".to_string());

        // from: some to: some; to overrides from
        let actual = get_merged_hashmaps(&Some(hashmap_1.clone()), &Some(hashmap_2.clone()))
            .expect("to be present");

        let mut expected: HashMap<String, String> = HashMap::<String, String>::new();
        expected.insert("k1".to_string(), "v1".to_string());
        expected.insert("k2".to_string(), "v2".to_string());
        assert_eq!(expected, actual);

        let mut hashmap_2: HashMap<String, String> = HashMap::<String, String>::new();
        hashmap_2.insert("k1".to_string(), "new1".to_string());
        hashmap_2.insert("k2".to_string(), "v2".to_string());

        let mut expected: HashMap<String, String> = HashMap::<String, String>::new();
        expected.insert("k1".to_string(), "new1".to_string());
        expected.insert("k2".to_string(), "v2".to_string());

        // from: some to: some; to overrides from
        let actual = get_merged_hashmaps(&Some(hashmap_1.clone()), &Some(hashmap_2.clone()))
            .expect("to be present");
        assert_eq!(expected, actual);

        let mut expected: HashMap<String, String> = HashMap::<String, String>::new();
        expected.insert("k1".to_string(), "v1".to_string());
        expected.insert("k2".to_string(), "v2".to_string());

        let actual = get_merged_hashmaps(&Some(hashmap_2.clone()), &Some(hashmap_1.clone()))
            .expect("to be present");
        assert_eq!(expected, actual);
        return Ok(());
    }

    #[test]
    fn test_mock() -> Result<()> {
        return Ok(());
    }
}
