use anyhow::{Context, Result};
use mockall_double::double;

use crate::helpers::utils::Paths;
#[double]
use crate::shell::editor::edit;
use crate::{helpers::operations::get_current_dir_toml, types::terrain::parse_terrain_from};

pub fn handle(paths: &Paths) -> Result<()> {
    let toml_file = get_current_dir_toml(paths).context("unable to get terrain.toml path")?;

    edit::file(&toml_file).context("failed to start editor")?;

    super::generate::generate_and_compile_all(parse_terrain_from(&toml_file)?, paths)?;

    Ok(())
}

#[cfg(test)]
mod test {

    use anyhow::Result;
    use mockall::predicate::eq;
    use serial_test::serial;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    use crate::helpers::utils::get_paths;
    use crate::{
        shell::{editor::mock_edit, zsh::mock_ops},
        types::{args::BiomeArg, terrain::test_data},
    };

    #[test]
    #[serial]
    fn handle_edit_opens_editor_and_compiles_scripts() -> Result<()> {
        let test_dir = tempdir()?;
        let test_dir_path: PathBuf = test_dir.path().into();
        let home_dir = tempdir()?;
        let home_dir_path: PathBuf = home_dir.path().into();

        let paths = get_paths(home_dir_path, test_dir_path)?;

        let mut terrain_toml_path: PathBuf = test_dir.path().into();
        terrain_toml_path.push("terrain.toml");

        std::fs::copy("./tests/data/terrain.full.toml", &terrain_toml_path)?;

        let mock_edit_file = mock_edit::file_context();
        mock_edit_file
            .expect()
            .with(eq(terrain_toml_path))
            .return_once(|_| Ok(()))
            .times(1);

        let home_dir_path: PathBuf = home_dir.path().into();
        let test_dir_path: PathBuf = test_dir.path().into();
        let scripts_dir_name = Path::canonicalize(test_dir_path.as_path())?
            .to_string_lossy()
            .to_string()
            .replace('/', "_");
        let scripts_dir_path = home_dir_path.join(PathBuf::from(
            ".config/terrainium/terrains/".to_owned() + &scripts_dir_name,
        ));
        let terrain = test_data::terrain_full();
        let main = terrain.get(Some(BiomeArg::None))?;
        let generate_and_compile_context = mock_ops::generate_and_compile_context();
        generate_and_compile_context
            .expect()
            .with(
                eq(scripts_dir_path.clone()),
                eq(String::from("none")),
                eq(main),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        let example_biome = terrain.get(Some(BiomeArg::Value("example_biome".to_owned())))?;
        generate_and_compile_context
            .expect()
            .with(
                eq(scripts_dir_path.clone()),
                eq(String::from("example_biome")),
                eq(example_biome),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        let example_biome2 = terrain.get(Some(BiomeArg::Value("example_biome2".to_owned())))?;
        generate_and_compile_context
            .expect()
            .with(
                eq(scripts_dir_path.clone()),
                eq(String::from("example_biome2")),
                eq(example_biome2),
            )
            .return_once(|_, _, _| Ok(()))
            .times(1);

        super::handle(&paths)?;

        Ok(())
    }
}
