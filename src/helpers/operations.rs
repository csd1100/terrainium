use std::collections::BTreeMap;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

use crate::helpers::utils::Paths;
use crate::types::args::BiomeArg;
use crate::types::terrain::parse_terrain_from;
use crate::types::terrain::Terrain;
use anyhow::{anyhow, Context, Result};
use mockall_double::double;

use super::constants::TERRAINIUM_TOML_PATH;

#[double]
use super::utils::misc;

pub fn create_config_dir(paths: &Paths) -> Result<()> {
    create_dir_if_not_exist(&get_central_store_path(paths)?)
        .context("unable to create terrains directory in terrainium config directory")?;
    Ok(())
}

pub fn get_terrain_name(cwd: &Path) -> String {
    let default_name = format!("some-terrain-{}", misc::get_uuid());
    cwd.file_name()
        .map_or(default_name, |val| val.to_string_lossy().to_string())
}

pub fn get_central_terrain_path(paths: &Paths) -> Result<PathBuf> {
    let mut dirname = get_central_store_path(paths)?;
    dirname.push("terrain.toml");
    Ok(dirname)
}

pub fn get_local_terrain_path(cwd: &Path) -> Result<PathBuf> {
    Ok(Path::join(cwd, "terrain.toml"))
}

pub fn is_terrain_present(paths: &Paths) -> Result<bool> {
    if is_local_terrain_present(paths.get_cwd())? || is_central_terrain_present(paths)? {
        return Ok(true);
    }
    Ok(false)
}

pub fn get_central_store_path(paths: &Paths) -> Result<PathBuf> {
    let terrain_dir = Path::canonicalize(paths.get_cwd().as_path())?
        .to_string_lossy()
        .to_string()
        .replace('/', "_");
    let dirname = Path::join(
        &get_terrainium_config_path(paths.get_home_dir())
            .context("unable to get terrainium config directory")?,
        "terrains",
    );
    let dirname = dirname.join(terrain_dir);
    Ok(dirname)
}

pub fn get_current_dir_toml(paths: &Paths) -> Result<PathBuf> {
    get_terrain_toml_path(false, paths)
}

fn get_active_terrain_toml(paths: &Paths) -> Result<PathBuf> {
    get_terrain_toml_path(true, paths)
}

pub fn get_terrain_toml_from_biome(biome: &Option<BiomeArg>, paths: &Paths) -> Result<PathBuf> {
    biome.as_ref().map_or(get_current_dir_toml(paths), |arg| {
        if let BiomeArg::Current(_) = arg {
            get_active_terrain_toml(paths)
        } else {
            get_current_dir_toml(paths)
        }
    })
}

pub fn get_parsed_terrain(paths: &Paths) -> Result<Terrain> {
    let toml_file = get_current_dir_toml(paths).context("unable to get terrain.toml path")?;
    parse_terrain_from(toml_file)
}

fn get_terrain_toml_path(get_active: bool, paths: &Paths) -> Result<PathBuf> {
    if get_active {
        if let Ok(toml_path) = std::env::var(TERRAINIUM_TOML_PATH) {
            return Ok(PathBuf::from(toml_path));
        }
    }

    if is_local_terrain_present(paths.get_cwd())
        .context("failed to check whether local terrain.toml exists")?
    {
        get_local_terrain_path(paths.get_cwd())
    } else if is_central_terrain_present(paths)
        .context("failed to check whether central terrain.toml exists")?
    {
        get_central_terrain_path(paths)
    } else {
        let err = anyhow!("unable to get terrain.toml for this project. initialize terrain with `terrainium init` command");
        Err(err)
    }
}

fn get_terrainium_config_path(home_dir: &Path) -> Result<PathBuf> {
    Ok(Path::join(
        &get_config_path(home_dir).context("unable to get config directory")?,
        "terrainium",
    ))
}

fn get_config_path(home_dir: &Path) -> Result<PathBuf> {
    let config_dir = Path::join(home_dir, PathBuf::from(".config"));
    create_dir_if_not_exist(&config_dir)?;
    Ok(config_dir)
}

fn is_local_terrain_present(cwd: &Path) -> Result<bool> {
    Ok(get_local_terrain_path(cwd)?.exists())
}

fn is_central_terrain_present(paths: &Paths) -> Result<bool> {
    Ok(get_central_terrain_path(paths)?.exists())
}

pub fn write_terrain(path: &Path, terrain: &Terrain) -> Result<()> {
    let contents: String = terrain.to_toml()?;
    Ok(std::fs::write(path, contents)?)
}

pub fn write_file(path: &Path, contents: String) -> Result<()> {
    Ok(std::fs::write(path, contents)?)
}

pub fn copy_file(from: &Path, to: &Path) -> Result<u64> {
    Ok(std::fs::copy(from, to)?)
}

pub fn create_dir_if_not_exist(dir: &Path) -> Result<bool> {
    if !dir.exists() {
        println!("creating a directory at path {}", dir.to_string_lossy());
        std::fs::create_dir_all(dir)?;
        return Ok(true);
    }
    Ok(false)
}

pub fn get_process_log_file(session_id: &String, filename: String) -> Result<(PathBuf, File)> {
    let tmp = PathBuf::from(format!("/tmp/terrainium-{}", session_id));
    create_dir_if_not_exist(&tmp)?;

    let mut out_path = tmp.clone();
    out_path.push(filename);
    let out = File::options()
        .append(true)
        .create_new(true)
        .open(&out_path)?;

    Ok((out_path, out))
}

pub fn remove_all_script_files(central_store: &Path) -> Result<()> {
    if let Ok(entries) = std::fs::read_dir(central_store) {
        for entry in entries {
            let Ok(entry) = entry else {
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

pub fn merge_maps(
    to: &BTreeMap<String, String>,
    from: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut return_map = to.clone();
    let _: Vec<_> = from
        .iter()
        .map(|(key, value)| return_map.insert(key.to_string(), value.to_string()))
        .collect();
    return_map
}

pub fn get_merged_maps(
    from: &Option<BTreeMap<String, String>>,
    to: &Option<BTreeMap<String, String>>,
) -> Option<BTreeMap<String, String>> {
    if from.is_some() && !to.is_some() {
        return from.clone();
    }
    if to.is_some() && !from.is_some() {
        return to.clone();
    }

    if let Some(from) = &from {
        if let Some(to) = &to {
            return Some(merge_maps(from, to));
        }
    }

    None
}

pub fn find_in_maps(
    from: &Option<BTreeMap<String, String>>,
    to_find: Vec<String>,
) -> Result<BTreeMap<String, Option<String>>> {
    let return_map: BTreeMap<String, Option<String>> = if let Some(values) = from {
        to_find
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
    use std::collections::BTreeMap;

    use anyhow::Result;

    use super::get_merged_maps;

    #[test]
    fn test_merge_maps() -> Result<()> {
        let mut map_1: BTreeMap<String, String> = BTreeMap::<String, String>::new();
        map_1.insert("k1".to_string(), "v1".to_string());

        // from: some to: none; from
        let actual = get_merged_maps(&Some(map_1.clone()), &None).expect("to be present");
        assert_eq!(map_1, actual);

        // from: none to: some; to
        let actual = get_merged_maps(&None, &Some(map_1.clone())).expect("to be present");
        assert_eq!(map_1, actual);

        let mut map_2: BTreeMap<String, String> = BTreeMap::<String, String>::new();
        map_2.insert("k2".to_string(), "v2".to_string());

        // from: some to: some; to overrides from
        let actual =
            get_merged_maps(&Some(map_1.clone()), &Some(map_2.clone())).expect("to be present");

        let mut expected: BTreeMap<String, String> = BTreeMap::<String, String>::new();
        expected.insert("k1".to_string(), "v1".to_string());
        expected.insert("k2".to_string(), "v2".to_string());
        assert_eq!(expected, actual);

        let mut map_2: BTreeMap<String, String> = BTreeMap::<String, String>::new();
        map_2.insert("k1".to_string(), "new1".to_string());
        map_2.insert("k2".to_string(), "v2".to_string());

        let mut expected: BTreeMap<String, String> = BTreeMap::<String, String>::new();
        expected.insert("k1".to_string(), "new1".to_string());
        expected.insert("k2".to_string(), "v2".to_string());

        // from: some to: some; to overrides from
        let actual =
            get_merged_maps(&Some(map_1.clone()), &Some(map_2.clone())).expect("to be present");
        assert_eq!(expected, actual);

        let mut expected: BTreeMap<String, String> = BTreeMap::<String, String>::new();
        expected.insert("k1".to_string(), "v1".to_string());
        expected.insert("k2".to_string(), "v2".to_string());

        let actual =
            get_merged_maps(&Some(map_2.clone()), &Some(map_1.clone())).expect("to be present");
        assert_eq!(expected, actual);
        Ok(())
    }

    #[test]
    fn test_mock() -> Result<()> {
        Ok(())
    }
}
