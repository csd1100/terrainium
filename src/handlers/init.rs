use anyhow::{anyhow, Context, Result};
use mockall_double::double;
use std::path::PathBuf;

use crate::{helpers::operations, types::terrain::Terrain};

#[double]
use crate::shell::editor::edit;

pub fn handle(central: bool, example: bool, edit: bool) -> Result<()> {
    if operations::is_terrain_present().context("failed to validate if terrain already exists")? {
        return Err(anyhow!(
            "terrain for this project is already present. edit existing terrain with `terrain edit` command"
        ));
    }

    operations::create_config_dir().context("unable to create config directory")?;

    let terrain_toml_path: PathBuf = if central {
        operations::get_central_terrain_path().context("unable to get central toml path")?
    } else {
        operations::get_local_terrain_path().context("unable to get local terrain.toml")?
    };

    let terrain: Terrain = if example {
        Terrain::example()
    } else {
        Terrain::new()
    };

    operations::write_terrain(&terrain_toml_path, &terrain)
        .context("failed to write generated terrain to toml file")?;

    println!(
        "terrain created at path {}",
        terrain_toml_path.to_string_lossy()
    );

    super::generate::generate_and_compile(terrain)?;

    if edit {
        println!("editing...");
        edit::file(&terrain_toml_path).context("failed to edit terrain.toml")?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use std::path::{Path, PathBuf};

    use anyhow::Result;
    use home::home_dir;
    use mockall::predicate::{always, eq};
    use serial_test::serial;
    use tempfile::tempdir;

    use crate::{
        helpers::utils::mock_fs,
        shell::zsh::mock_ops,
        types::{args::BiomeArg, terrain::Terrain},
    };

    #[test]
    #[serial]
    fn init_without_any_options_creates_and_compiles_terrain() -> Result<()> {
        let test_dir = tempdir()?;

        let home_dir = home_dir().unwrap();

        let test_dir_path: PathBuf = test_dir.path().into();
        let mock_cwd = mock_fs::get_cwd_context();
        mock_cwd
            .expect()
            .returning(move || {
                let test_dir_path: PathBuf = test_dir_path.clone();
                Ok(test_dir_path)
            })
            .times(5);

        let mut terrains_dir = home_dir.clone();
        terrains_dir.push(".config/terrainium/terrains");
        let test_dir_path: PathBuf = test_dir.path().into();
        let scripts_dir_name = Path::canonicalize(test_dir_path.as_path())?
            .to_string_lossy()
            .to_string()
            .replace('/', "_");
        let mut scripts_dir = terrains_dir;
        scripts_dir.push(scripts_dir_name);

        let mock_create_dir_ctx = mock_fs::create_dir_if_not_exist_context();
        mock_create_dir_ctx
            .expect()
            .with(eq(scripts_dir.clone()))
            .return_once(|_| Ok(false))
            .times(1);

        let mut test_terrain_path: PathBuf = test_dir.path().into();
        test_terrain_path.push("terrain.toml");
        let mock_write = mock_fs::write_file_context();
        mock_write
            .expect()
            .with(eq(test_terrain_path), always())
            .return_once(|_, _| Ok(()));

        let remove_all_script_files = mock_fs::remove_all_script_files_context();
        remove_all_script_files
            .expect()
            .with(eq(scripts_dir.clone()))
            .return_once(|_| Ok(()))
            .times(1);

        let terrain = Terrain::new().get(Some(BiomeArg::None))?;
        let generate_and_compile_context = mock_ops::generate_and_compile_context();
        generate_and_compile_context
            .expect()
            .with(
                eq(scripts_dir),
                eq(String::from("none")),
                eq(terrain),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        super::handle(false, false, false)?;

        Ok(())
    }
    //
    //     #[test]
    //     #[serial]
    //     fn init_with_full_creates_and_compiles_terrain() -> Result<()> {
    //         let mock_create_dir_ctx = mock_fs::create_config_dir_context();
    //         mock_create_dir_ctx.expect().return_once(|| Ok(())).times(1);
    //
    //         let mock_get_local_terrain_ctx = mock_fs::get_local_terrain_path_context();
    //         mock_get_local_terrain_ctx
    //             .expect()
    //             .return_once(|| Ok(PathBuf::from("/tmp/terrainium-init-test/terrain.toml")))
    //             .times(1);
    //
    //         let is_terrain_present_context = mock_fs::is_terrain_present_context();
    //         is_terrain_present_context
    //             .expect()
    //             .return_once(|| Ok(false))
    //             .times(1);
    //
    //         let write_terrain_context = mock_fs::write_terrain_context();
    //         write_terrain_context
    //             .expect()
    //             .with(
    //                 eq(PathBuf::from("/tmp/terrainium-init-test/terrain.toml")),
    //                 eq(Terrain::example()),
    //             )
    //             .return_once(|_, _| Ok(()))
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
    //         let terrain = Terrain::example();
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
    //         super::handle(false, true, false)?;
    //
    //         Ok(())
    //     }
    //
    //     #[test]
    //     #[serial]
    //     fn init_with_central_creates_and_compiles_terrain() -> Result<()> {
    //         let mock_create_dir_ctx = mock_fs::create_config_dir_context();
    //         mock_create_dir_ctx.expect().return_once(|| Ok(())).times(1);
    //
    //         let mock_get_central_terrain_ctx = mock_fs::get_central_terrain_path_context();
    //         mock_get_central_terrain_ctx
    //             .expect()
    //             .return_once(|| Ok(PathBuf::from("/tmp/terrainium-init-test/terrain.toml")))
    //             .times(1);
    //
    //         let is_terrain_present_context = mock_fs::is_terrain_present_context();
    //         is_terrain_present_context
    //             .expect()
    //             .return_once(|| Ok(false))
    //             .times(1);
    //
    //         let write_terrain_context = mock_fs::write_terrain_context();
    //         write_terrain_context
    //             .expect()
    //             .with(
    //                 eq(PathBuf::from("/tmp/terrainium-init-test/terrain.toml")),
    //                 eq(Terrain::new()),
    //             )
    //             .return_once(|_, _| Ok(()))
    //             .times(1);
    //
    //         let get_central_store_path_context = mock_fs::get_central_store_path_context();
    //         get_central_store_path_context
    //             .expect()
    //             .return_once(|| Ok(PathBuf::from("~/.config/terrainium/terrains/")));
    //
    //         let remove_all_script_files = mock_fs::remove_all_script_files_context();
    //         remove_all_script_files
    //             .expect()
    //             .withf(|path| path == PathBuf::from("~/.config/terrainium/terrains/").as_path())
    //             .return_once(|_| Ok(()))
    //             .times(1);
    //
    //         let terrain = Terrain::new().get(Some(BiomeArg::None))?;
    //         let generate_and_compile_context = mock_ops::generate_and_compile_context();
    //         generate_and_compile_context
    //             .expect()
    //             .with(
    //                 eq(PathBuf::from("~/.config/terrainium/terrains/")),
    //                 eq(String::from("none")),
    //                 eq(terrain),
    //             )
    //             .return_once(|_, _, _| Ok(()))
    //             .times(1);
    //
    //         super::handle(true, false, false)?;
    //
    //         Ok(())
    //     }
    //
    //     #[test]
    //     #[serial]
    //     fn init_with_edit_creates_and_compiles_terrain_and_starts_editor() -> Result<()> {
    //         let mock_create_dir_ctx = mock_fs::create_config_dir_context();
    //         mock_create_dir_ctx.expect().return_once(|| Ok(())).times(1);
    //
    //         let mock_get_local_terrain_ctx = mock_fs::get_local_terrain_path_context();
    //         mock_get_local_terrain_ctx
    //             .expect()
    //             .return_once(|| Ok(PathBuf::from("/tmp/terrainium-init-test/terrain.toml")))
    //             .times(1);
    //
    //         let is_terrain_present_context = mock_fs::is_terrain_present_context();
    //         is_terrain_present_context
    //             .expect()
    //             .return_once(|| Ok(false))
    //             .times(1);
    //
    //         let write_terrain_context = mock_fs::write_terrain_context();
    //         write_terrain_context
    //             .expect()
    //             .with(
    //                 eq(PathBuf::from("/tmp/terrainium-init-test/terrain.toml")),
    //                 eq(Terrain::new()),
    //             )
    //             .return_once(|_, _| Ok(()))
    //             .times(1);
    //
    //         let get_central_store_path_context = mock_fs::get_central_store_path_context();
    //         get_central_store_path_context
    //             .expect()
    //             .return_once(|| Ok(PathBuf::from("~/.config/terrainium/terrains/")));
    //
    //         let remove_all_script_files = mock_fs::remove_all_script_files_context();
    //         remove_all_script_files
    //             .expect()
    //             .withf(|path| path == PathBuf::from("~/.config/terrainium/terrains/").as_path())
    //             .return_once(|_| Ok(()))
    //             .times(1);
    //
    //         let terrain = Terrain::new().get(Some(BiomeArg::None))?;
    //         let generate_and_compile_context = mock_ops::generate_and_compile_context();
    //         generate_and_compile_context
    //             .expect()
    //             .with(
    //                 eq(PathBuf::from("~/.config/terrainium/terrains/")),
    //                 eq(String::from("none")),
    //                 eq(terrain),
    //             )
    //             .return_once(|_, _, _| Ok(()))
    //             .times(1);
    //
    //         let mock_edit_file = mock_edit::file_context();
    //         mock_edit_file
    //             .expect()
    //             .with(eq(PathBuf::from("/tmp/terrainium-init-test/terrain.toml")))
    //             .return_once(|_| Ok(()))
    //             .times(1);
    //
    //         super::handle(false, false, true)?;
    //
    //         Ok(())
    //     }
    //
    //     #[test]
    //     #[serial]
    //     fn init_with_terrain_present_throws_error() -> Result<()> {
    //         let mock_is_present = mock_fs::is_terrain_present_context();
    //         mock_is_present.expect().return_once(|| Ok(true));
    //
    //         let err = super::handle(false, false, false).unwrap_err().to_string();
    //
    //         assert_eq!(err, "terrain for this project is already present. edit existing terrain with `terrain edit` command");
    //
    //         Ok(())
    //     }
}
