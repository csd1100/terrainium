use anyhow::{anyhow, Result};
use mockall_double::double;

use crate::helpers::utils::Paths;
#[double]
use crate::shell::zsh::ops;
use crate::{
    helpers::operations::{get_central_store_path, get_current_dir_toml, remove_all_script_files},
    types::terrain::{parse_terrain_from, Terrain},
};

pub fn handle(paths: &Paths) -> Result<()> {
    let terrain = parse_terrain_from(get_current_dir_toml(paths)?)?;
    generate_and_compile_all(terrain, paths)
}

pub fn generate_and_compile_all(terrain: Terrain, paths: &Paths) -> Result<()> {
    let central_store = get_central_store_path(paths)?;

    remove_all_script_files(central_store.as_path())?;

    let result: Result<Vec<_>> = terrain
        .into_iter()
        .map(|(biome_name, environment)| {
            ops::generate_and_compile(&central_store, biome_name, environment)
        })
        .collect();

    if result.is_err() {
        Err(anyhow!(format!(
            "Error while generating and compiling scripts, error: {}",
            result.unwrap_err()
        )))
    } else {
        Ok(())
    }
}

// #[cfg(test)]
// mod test {
//     use std::path::PathBuf;
//
//     use anyhow::Result;
//     use mockall::predicate::eq;
//     use serial_test::serial;
//
//     use crate::{
//         helpers::operations::mock_fs,
//         shell::zsh::mock_ops,
//         types::{args::BiomeArg, terrain::test_data},
//     };
//
//     #[test]
//     #[serial]
//     fn handle_generate_generates_scripts() -> Result<()> {
//         let mock_get_toml_path = mock_fs::get_current_dir_toml_context();
//         mock_get_toml_path
//             .expect()
//             .return_once(|| Ok(PathBuf::from("./example_configs/terrain.full.toml")))
//             .times(1);
//
//         let get_central_store_path_context = mock_fs::get_central_store_path_context();
//         get_central_store_path_context
//             .expect()
//             .return_once(|| Ok(PathBuf::from("~/.config/terrainium/terrains/")))
//             .times(1);
//
//         let remove_all_script_files = mock_fs::remove_all_script_files_context();
//         remove_all_script_files
//             .expect()
//             .withf(|path| path == PathBuf::from("~/.config/terrainium/terrains/").as_path())
//             .return_once(|_| Ok(()))
//             .times(1);
//
//         let terrain = test_data::terrain_full();
//         let main = terrain.get(Some(BiomeArg::None))?;
//         let generate_and_compile_context = mock_ops::generate_and_compile_context();
//         generate_and_compile_context
//             .expect()
//             .with(
//                 eq(PathBuf::from("~/.config/terrainium/terrains/")),
//                 eq(String::from("none")),
//                 eq(main),
//             )
//             .return_once(|_, _, _| Ok(()))
//             .times(1);
//
//         let example_biome = terrain.get(Some(BiomeArg::Value("example_biome".to_owned())))?;
//         generate_and_compile_context
//             .expect()
//             .with(
//                 eq(PathBuf::from("~/.config/terrainium/terrains/")),
//                 eq(String::from("example_biome")),
//                 eq(example_biome),
//             )
//             .return_once(|_, _, _| Ok(()))
//             .times(1);
//
//         let example_biome2 = terrain.get(Some(BiomeArg::Value("example_biome2".to_owned())))?;
//         generate_and_compile_context
//             .expect()
//             .with(
//                 eq(PathBuf::from("~/.config/terrainium/terrains/")),
//                 eq(String::from("example_biome2")),
//                 eq(example_biome2),
//             )
//             .return_once(|_, _, _| Ok(()))
//             .times(1);
//
//         super::handle()?;
//
//         Ok(())
//     }
// }
