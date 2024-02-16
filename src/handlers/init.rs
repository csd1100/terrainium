use anyhow::{anyhow, Context, Result};
use mockall_double::double;
use std::path::PathBuf;

use crate::types::terrain::Terrain;

#[double]
use crate::shell::editor::edit;

#[double]
use crate::shell::zsh::ops;

#[double]
use crate::helpers::helpers::fs;

pub fn handle(central: bool, full: bool, edit: bool) -> Result<()> {
    if !fs::is_terrain_present().context("failed to validate if terrain already exists")? {
        fs::create_config_dir().context("unable to create config directory")?;

        let terrain_toml_path: PathBuf = if central {
            fs::get_central_terrain_path().context("unable to get central toml path")?
        } else {
            fs::get_local_terrain_path().context("unable to get local terrain.toml")?
        };

        let terrain: Terrain = if full {
            Terrain::default()
        } else {
            Terrain::new()
        };

        fs::write_terrain(&terrain_toml_path, &terrain)
            .context("failed to write generated terrain to toml file")?;

        println!(
            "terrain created at path {}",
            terrain_toml_path.to_string_lossy()
        );

        let central_store =
            fs::get_central_store_path().context("unable to get central store path")?;
        let result: Result<Vec<_>> = terrain
            .into_iter()
            .map(|(biome_name, environment)| {
                ops::generate_and_compile(&central_store, biome_name, environment)
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
            "terrain for this project is already present. edit existing terrain with `terrain edit` command"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use anyhow::{Ok, Result};
    use mockall::predicate::eq;
    use serial_test::serial;

    use crate::{
        helpers::helpers::mock_fs,
        shell::{editor::mock_edit, zsh::mock_ops},
        types::{args::BiomeArg, terrain::Terrain},
    };

    #[test]
    #[serial]
    fn init_without_any_options_creates_and_compiles_terrain() -> Result<()> {
        let mock_create_dir_ctx = mock_fs::create_config_dir_context();
        mock_create_dir_ctx
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/")))
            .times(1);

        let mock_get_local_terrain_ctx = mock_fs::get_local_terrain_path_context();
        mock_get_local_terrain_ctx
            .expect()
            .return_once(|| {
                Ok(PathBuf::from(
                    "./example_configs/terrain.without.biomes.toml",
                ))
            })
            .times(1);

        let is_terrain_present_context = mock_fs::is_terrain_present_context();
        is_terrain_present_context
            .expect()
            .return_once(|| Ok(false))
            .times(1);

        let write_terrain_context = mock_fs::write_terrain_context();
        write_terrain_context
            .expect()
            .with(
                eq(PathBuf::from(
                    "./example_configs/terrain.without.biomes.toml",
                )),
                eq(Terrain::new()),
            )
            .return_once(|_, _| Ok(()))
            .times(1);

        let get_central_store_path_context = mock_fs::get_central_store_path_context();
        get_central_store_path_context
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/terrains/")));

        let terrain = Terrain::new().get(Some(BiomeArg::None))?;
        let generate_and_compile_context = mock_ops::generate_and_compile_context();
        generate_and_compile_context
            .expect()
            .with(
                eq(PathBuf::from("~/.config/terrainium/terrains/")),
                eq(String::from("none")),
                eq(terrain),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        super::handle(false, false, false)?;

        Ok(())
    }

    #[test]
    #[serial]
    fn init_with_full_creates_and_compiles_terrain() -> Result<()> {
        let mock_create_dir_ctx = mock_fs::create_config_dir_context();
        mock_create_dir_ctx
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/")))
            .times(1);

        let mock_get_local_terrain_ctx = mock_fs::get_local_terrain_path_context();
        mock_get_local_terrain_ctx
            .expect()
            .return_once(|| Ok(PathBuf::from("./example_configs/terrain.full.toml")))
            .times(1);

        let is_terrain_present_context = mock_fs::is_terrain_present_context();
        is_terrain_present_context
            .expect()
            .return_once(|| Ok(false))
            .times(1);

        // toml deserializes vec in different order so string equals will fail
        // so just check if called with any string containing default_biome
        let write_terrain_context = mock_fs::write_terrain_context();
        write_terrain_context
            .expect()
            .with(
                eq(PathBuf::from("./example_configs/terrain.full.toml")),
                eq(Terrain::default()),
            )
            .return_once(|_, _| Ok(()))
            .times(1);

        let get_central_store_path_context = mock_fs::get_central_store_path_context();
        get_central_store_path_context
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/terrains/")))
            .times(1);

        let terrain = Terrain::default();
        let main = terrain.get(Some(BiomeArg::None))?;
        let generate_and_compile_context = mock_ops::generate_and_compile_context();
        generate_and_compile_context
            .expect()
            .with(
                eq(PathBuf::from("~/.config/terrainium/terrains/")),
                eq(String::from("none")),
                eq(main),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        let generate_and_compile_context = mock_ops::generate_and_compile_context();
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
        super::handle(false, true, false)?;

        Ok(())
    }

    #[test]
    #[serial]
    fn init_with_central_creates_and_compiles_terrain() -> Result<()> {
        let mock_create_dir_ctx = mock_fs::create_config_dir_context();
        mock_create_dir_ctx
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/")))
            .times(1);

        let mock_get_central_terrain_ctx = mock_fs::get_central_terrain_path_context();
        mock_get_central_terrain_ctx
            .expect()
            .return_once(|| {
                Ok(PathBuf::from(
                    "./example_configs/terrain.without.biomes.toml",
                ))
            })
            .times(1);

        let is_terrain_present_context = mock_fs::is_terrain_present_context();
        is_terrain_present_context
            .expect()
            .return_once(|| Ok(false))
            .times(1);

        let write_terrain_context = mock_fs::write_terrain_context();
        write_terrain_context
            .expect()
            .with(
                eq(PathBuf::from(
                    "./example_configs/terrain.without.biomes.toml",
                )),
                eq(Terrain::new()),
            )
            .return_once(|_, _| Ok(()))
            .times(1);

        let get_central_store_path_context = mock_fs::get_central_store_path_context();
        get_central_store_path_context
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/terrains/")));

        let terrain = Terrain::new().get(Some(BiomeArg::None))?;
        let generate_and_compile_context = mock_ops::generate_and_compile_context();
        generate_and_compile_context
            .expect()
            .with(
                eq(PathBuf::from("~/.config/terrainium/terrains/")),
                eq(String::from("none")),
                eq(terrain),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        super::handle(true, false, false)?;

        Ok(())
    }

    #[test]
    #[serial]
    fn init_with_edit_creates_and_compiles_terrain_and_starts_editor() -> Result<()> {
        let mock_create_dir_ctx = mock_fs::create_config_dir_context();
        mock_create_dir_ctx
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/")))
            .times(1);

        let mock_get_local_terrain_ctx = mock_fs::get_local_terrain_path_context();
        mock_get_local_terrain_ctx
            .expect()
            .return_once(|| {
                Ok(PathBuf::from(
                    "./example_configs/terrain.without.biomes.toml",
                ))
            })
            .times(1);

        let is_terrain_present_context = mock_fs::is_terrain_present_context();
        is_terrain_present_context
            .expect()
            .return_once(|| Ok(false))
            .times(1);

        let write_terrain_context = mock_fs::write_terrain_context();
        write_terrain_context
            .expect()
            .with(
                eq(PathBuf::from(
                    "./example_configs/terrain.without.biomes.toml",
                )),
                eq(Terrain::new()),
            )
            .return_once(|_, _| Ok(()))
            .times(1);

        let get_central_store_path_context = mock_fs::get_central_store_path_context();
        get_central_store_path_context
            .expect()
            .return_once(|| Ok(PathBuf::from("~/.config/terrainium/terrains/")));

        let terrain = Terrain::new().get(Some(BiomeArg::None))?;
        let generate_and_compile_context = mock_ops::generate_and_compile_context();
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

        super::handle(false, false, true)?;

        Ok(())
    }

    #[test]
    #[serial]
    fn init_with_terrain_present_throws_error() -> Result<()> {
        let mock_is_present = mock_fs::is_terrain_present_context();
        mock_is_present.expect().return_once(|| Ok(true));

        let err = super::handle(false, false, false).unwrap_err().to_string();

        assert_eq!(err, "terrain for this project is already present. edit existing terrain with `terrain edit` command");

        Ok(())
    }
}
