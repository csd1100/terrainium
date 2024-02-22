use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Ok, Result};
use home::home_dir;

#[cfg(test)]
use mockall::automock;

use crate::types::{errors::TerrainiumErrors, terrain::parse_terrain};

use super::constants::TERRAINIUM_TOML_PATH;

#[cfg_attr(test, automock)]
pub mod fs {
    use std::{
        fs::File,
        path::{Path, PathBuf},
    };

    use anyhow::{Context, Ok, Result};

    use super::get_terrainium_config_path;
    use crate::types::{args::BiomeArg, terrain::Terrain};

    pub fn create_config_dir() -> Result<PathBuf> {
        let config_path =
            get_terrainium_config_path().context("unable to get terrainium config path")?;
        println!("[config_path: {:?}]\n", config_path);
        super::create_dir_if_not_exist(config_path.as_path())
            .context("unable to create terrainium config directory")?;
        super::create_dir_if_not_exist(get_central_store_path()?.as_path())
            .context("unable to create terrains directory in terrainium config directory")?;
        Ok(config_path)
    }

    pub fn get_terrain_name() -> String {
        let dir = std::env::current_dir();
        let default_name = format!("some-terrain-{}", uuid::Uuid::new_v4());
        match dir {
            std::result::Result::Ok(cwd) => cwd
                .file_name()
                .map_or(default_name, |val| val.to_string_lossy().to_string()),
            Err(_) => default_name,
        }
    }

    pub fn get_central_terrain_path() -> Result<PathBuf> {
        let mut dirname = get_central_store_path()?;
        dirname.push("terrain.toml");
        Ok(dirname)
    }

    pub fn get_local_terrain_path() -> Result<PathBuf> {
        Ok(Path::join(&std::env::current_dir()?, "terrain.toml"))
    }

    pub fn is_terrain_present() -> Result<bool> {
        if super::is_local_terrain_present()? || super::is_central_terrain_present()? {
            return Ok(true);
        }
        Ok(false)
    }

    pub fn get_central_store_path() -> Result<PathBuf> {
        let cwd = std::env::current_dir()?;

        let terrain_dir = Path::canonicalize(cwd.as_path())?
            .to_string_lossy()
            .to_string()
            .replace('/', "_");
        let dirname = Path::join(
            &get_terrainium_config_path().context("unable to get terrainium config directory")?,
            "terrains",
        );
        let dirname = dirname.join(terrain_dir);
        Ok(dirname)
    }

    pub fn get_current_dir_toml() -> Result<PathBuf> {
        super::get_terrain_toml_path(false)
    }

    fn get_active_terrain_toml() -> Result<PathBuf> {
        super::get_terrain_toml_path(true)
    }

    pub fn get_terrain_toml_from_biome(biome: &Option<BiomeArg>) -> Result<PathBuf> {
        biome.as_ref().map_or(get_current_dir_toml(), |arg| {
            if let BiomeArg::Current(_) = arg {
                get_active_terrain_toml()
            } else {
                get_current_dir_toml()
            }
        })
    }

    pub fn write_file(path: &Path, contents: String) -> Result<()> {
        Ok(std::fs::write(path, contents)?)
    }

    pub fn write_terrain(path: &Path, terrain: &Terrain) -> Result<()> {
        write_file(path, terrain.to_toml()?)
    }

    pub fn copy_file(from: &Path, to: &Path) -> Result<u64> {
        Ok(std::fs::copy(from, to)?)
    }

    pub fn get_parsed_terrain() -> Result<Terrain> {
        let toml_file = get_current_dir_toml().context("unable to get terrain.toml path")?;
        super::parse_terrain(&toml_file)
    }

    pub fn get_process_log_file(session_id: &String, filename: String) -> Result<(PathBuf, File)> {
        let tmp = PathBuf::from(format!("/tmp/terrainium-{}", session_id));
        super::create_dir_if_not_exist(&tmp)?;

        let mut out_path = tmp.clone();
        out_path.push(filename);
        let out = File::options()
            .append(true)
            .create_new(true)
            .open(&out_path)?;

        Ok((out_path, out))
    }

    pub fn remove_all_script_files(central_store: &Path) -> Result<()> {
        if let std::result::Result::Ok(entries) = std::fs::read_dir(central_store) {
            for entry in entries {
                let std::result::Result::Ok(entry) = entry else {
                    continue;
                };
                if let Some(ext) = entry.path().extension() {
                    if ext.to_str() == Some("zwc") || ext.to_str() == Some("zsh") {
                        std::fs::remove_file(entry.path())?;
                    }
                };
            }
        }
        Ok(())
    }
}

fn get_terrain_toml_path(active: bool) -> Result<PathBuf> {
    if active {
        if let std::result::Result::Ok(toml_path) = std::env::var(TERRAINIUM_TOML_PATH) {
            return Ok(PathBuf::from(toml_path));
        }
    }

    if is_local_terrain_present().context("failed to check whether local terrain.toml exists")? {
        fs::get_local_terrain_path()
    } else if is_central_terrain_present()
        .context("failed to check whether central terrain.toml exists")?
    {
        return fs::get_central_terrain_path();
    } else {
        let err = anyhow!("unable to get terrain.toml for this project. initialize terrain with `terrainium init` command");
        return Err(err);
    }
}

pub fn create_dir_if_not_exist(dir: &Path) -> Result<bool> {
    if !Path::try_exists(dir)? {
        println!("creating a directory at path {}", dir.to_string_lossy());
        std::fs::create_dir_all(dir)?;
        return Ok(true);
    }
    Ok(false)
}

fn get_terrainium_config_path() -> Result<PathBuf> {
    Ok(Path::join(
        &get_config_path().context("unable to get config directory")?,
        "terrainium",
    ))
}

fn get_config_path() -> Result<PathBuf> {
    let home = home_dir();
    if let Some(home) = home {
        if Path::is_dir(home.as_path()) {
            let config_dir = Path::join(&home, PathBuf::from(".config"));
            if Path::try_exists(config_dir.as_path())? {
                return Ok(config_dir);
            }
            return Ok(home);
        } else {
            return Err(TerrainiumErrors::InvalidHomeDirectory.into());
        }
    }
    Err(TerrainiumErrors::UnableToFindHome.into())
}

fn is_local_terrain_present() -> Result<bool> {
    Ok(Path::try_exists(&fs::get_local_terrain_path()?)?)
}

fn is_central_terrain_present() -> Result<bool> {
    Ok(Path::try_exists(&fs::get_central_terrain_path()?)?)
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
    return_map
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

    None
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
                    (env.to_string(), Some(value.to_string()))
                } else {
                    (env.to_string(), None)
                }
            })
            .collect()
    } else {
        return Err(anyhow!("Not Defined"));
    };
    Ok(return_map)
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
        Ok(())
    }

    #[test]
    fn test_mock() -> Result<()> {
        Ok(())
    }
}
