use anyhow::{anyhow, Context, Result};
use mockall_double::double;
use std::path::PathBuf;

use crate::types::terrain::Terrain;

#[double]
use crate::shell::editor::edit;

#[double]
use crate::shell::zsh::ZshOps;

#[double]
use crate::handlers::helpers::FS;

pub fn handle_init(central: bool, full: bool, edit: bool) -> Result<()> {
    FS::create_config_dir().context("unable to create config directory")?;

    let terrain_toml_path: PathBuf = if central {
        FS::get_central_terrain_path().context("unable to get central toml path")?
    } else {
        FS::get_local_terrain_path().context("unable to get local terrain.toml")?
    };

    if !FS::is_terrain_present().context("failed to validate if terrain already exists")? {
        let terrain: Terrain;
        if full {
            terrain = Terrain::default();
        } else {
            terrain = Terrain::new();
        }

        let contents = terrain.to_toml()?;
        FS::write_file(&terrain_toml_path, contents)
            .context("failed to write generated terrain to toml file")?;

        println!(
            "terrain created at path {}",
            terrain_toml_path.to_string_lossy().to_string()
        );

        let central_store =
            FS::get_central_store_path().context("unable to get central store path")?;
        let result: Result<Vec<_>> = terrain
            .into_iter()
            .map(|(biome_name, environment)| {
                ZshOps::generate_and_compile(&central_store, biome_name, environment)
            })
            .collect();

        if result.is_err() {
            return Err(anyhow!(format!(
                "Error while generating and compiling scripts, error: {}",
                result.unwrap_err()
            )));
        }

        if edit {
            println!("editing...");
            edit::file(&terrain_toml_path).context("failed to edit terrain.toml")?;
        }
    } else {
        return Err(anyhow!(
            "terrain for this project is already present. edit existing with `terrain edit` command."
        ));
    }

    return Ok(());
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use anyhow::{Ok, Result};
    use mockall::predicate::eq;
    use std::sync::Mutex;

    use crate::{
        handlers::helpers::MockFS,
        shell::{editor::mock_edit, zsh::MockZshOps},
        types::{args::BiomeArg, terrain::Terrain},
    };

    use super::handle_init;

    static MTX: Mutex<()> = Mutex::new(());

    #[test]
    fn init_without_any_options_creates_and_compiles_terrain() -> Result<()> {
        let _m = MTX.lock();

        let mock_create_dir_ctx = MockFS::create_config_dir_context();
        mock_create_dir_ctx
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/")))
            .times(1);

        let mock_get_local_terrain_ctx = MockFS::get_local_terrain_path_context();
        mock_get_local_terrain_ctx
            .expect()
            .return_once(|| {
                Ok(PathBuf::from(
                    "./example_configs/terrain.without.biomes.toml",
                ))
            })
            .times(1);

        let is_terrain_present_context = MockFS::is_terrain_present_context();
        is_terrain_present_context
            .expect()
            .return_once(|| Ok(false))
            .times(1);

        let contents = Terrain::new().to_toml()?;

        let write_file_context = MockFS::write_file_context();
        write_file_context
            .expect()
            .with(
                eq(PathBuf::from(
                    "./example_configs/terrain.without.biomes.toml",
                )),
                eq(contents),
            )
            .return_once(|_, _| Ok(()))
            .times(1);

        let get_central_store_path_context = MockFS::get_central_store_path_context();
        get_central_store_path_context
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/terrains/")));

        let terrain = Terrain::new().get(Some(BiomeArg::None))?;
        let generate_and_compile_context = MockZshOps::generate_and_compile_context();
        generate_and_compile_context
            .expect()
            .with(
                eq(PathBuf::from("~/.config/terrainium/terrains/")),
                eq(String::from("none")),
                eq(terrain),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        handle_init(false, false, false)?;

        return Ok(());
    }

    #[test]
    fn init_with_full_creates_and_compiles_terrain() -> Result<()> {
        let _m = MTX.lock();
        let mock_create_dir_ctx = MockFS::create_config_dir_context();
        mock_create_dir_ctx
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/")))
            .times(1);

        let mock_get_local_terrain_ctx = MockFS::get_local_terrain_path_context();
        mock_get_local_terrain_ctx
            .expect()
            .return_once(|| Ok(PathBuf::from("./example_configs/terrain.full.toml")))
            .times(1);

        let is_terrain_present_context = MockFS::is_terrain_present_context();
        is_terrain_present_context
            .expect()
            .return_once(|| Ok(false))
            .times(1);

        // toml deserializes vec in different order so string equals will fail
        // so just check if called with any string containing default_biome
        let write_file_context = MockFS::write_file_context();
        write_file_context
            .expect()
            .withf(|path, contents| {
                return path == PathBuf::from("./example_configs/terrain.full.toml")
                    && contents.contains("default_biome = \"example_biome\"");
            })
            .return_once(|_, _| Ok(()))
            .times(1);

        let get_central_store_path_context = MockFS::get_central_store_path_context();
        get_central_store_path_context
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/terrains/")));

        let terrain = Terrain::default();
        let main = terrain.get(Some(BiomeArg::None))?;
        let generate_and_compile_context = MockZshOps::generate_and_compile_context();
        generate_and_compile_context
            .expect()
            .with(
                eq(PathBuf::from("~/.config/terrainium/terrains/")),
                eq(String::from("none")),
                eq(main),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        let generate_and_compile_context = MockZshOps::generate_and_compile_context();
        let example_biome = terrain.get(Some(BiomeArg::Value("example_biome".to_owned())))?;
        generate_and_compile_context
            .expect()
            .with(
                eq(PathBuf::from("~/.config/terrainium/terrains/")),
                eq(String::from("example_biome")),
                eq(example_biome),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);
        handle_init(false, true, false)?;

        return Ok(());
    }

    #[test]
    fn init_with_central_creates_and_compiles_terrain() -> Result<()> {
        let _m = MTX.lock();

        let mock_create_dir_ctx = MockFS::create_config_dir_context();
        mock_create_dir_ctx
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/")))
            .times(1);

        let mock_get_central_terrain_ctx = MockFS::get_central_terrain_path_context();
        mock_get_central_terrain_ctx
            .expect()
            .return_once(|| {
                Ok(PathBuf::from(
                    "./example_configs/terrain.without.biomes.toml",
                ))
            })
            .times(1);

        let is_terrain_present_context = MockFS::is_terrain_present_context();
        is_terrain_present_context
            .expect()
            .return_once(|| Ok(false))
            .times(1);

        let contents = Terrain::new().to_toml()?;

        let write_file_context = MockFS::write_file_context();
        write_file_context
            .expect()
            .with(
                eq(PathBuf::from(
                    "./example_configs/terrain.without.biomes.toml",
                )),
                eq(contents),
            )
            .return_once(|_, _| Ok(()))
            .times(1);

        let get_central_store_path_context = MockFS::get_central_store_path_context();
        get_central_store_path_context
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/terrains/")));

        let terrain = Terrain::new().get(Some(BiomeArg::None))?;
        let generate_and_compile_context = MockZshOps::generate_and_compile_context();
        generate_and_compile_context
            .expect()
            .with(
                eq(PathBuf::from("~/.config/terrainium/terrains/")),
                eq(String::from("none")),
                eq(terrain),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        handle_init(true, false, false)?;

        return Ok(());
    }

    #[test]
    fn init_with_edit_creates_and_compiles_terrain_and_starts_editor() -> Result<()> {
        let _m = MTX.lock();

        let mock_create_dir_ctx = MockFS::create_config_dir_context();
        mock_create_dir_ctx
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/")))
            .times(1);

        let mock_get_local_terrain_ctx = MockFS::get_local_terrain_path_context();
        mock_get_local_terrain_ctx
            .expect()
            .return_once(|| {
                Ok(PathBuf::from(
                    "./example_configs/terrain.without.biomes.toml",
                ))
            })
            .times(1);

        let is_terrain_present_context = MockFS::is_terrain_present_context();
        is_terrain_present_context
            .expect()
            .return_once(|| Ok(false))
            .times(1);

        let contents = Terrain::new().to_toml()?;

        let write_file_context = MockFS::write_file_context();
        write_file_context
            .expect()
            .with(
                eq(PathBuf::from(
                    "./example_configs/terrain.without.biomes.toml",
                )),
                eq(contents),
            )
            .return_once(|_, _| Ok(()))
            .times(1);

        let get_central_store_path_context = MockFS::get_central_store_path_context();
        get_central_store_path_context
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/terrains/")));

        let terrain = Terrain::new().get(Some(BiomeArg::None))?;
        let generate_and_compile_context = MockZshOps::generate_and_compile_context();
        generate_and_compile_context
            .expect()
            .with(
                eq(PathBuf::from("~/.config/terrainium/terrains/")),
                eq(String::from("none")),
                eq(terrain),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        let mock_edit_file = mock_edit::file_context();
        mock_edit_file
            .expect()
            .with(eq(PathBuf::from(
                "./example_configs/terrain.without.biomes.toml",
            )))
            .return_once(|_| Ok(()))
            .times(1);

        handle_init(false, false, true)?;

        return Ok(());
    }
}
