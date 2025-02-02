use crate::client::shell::Shell;
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use anyhow::{Context as AnyhowContext, Result};
use std::fs::{create_dir_all, exists, read_to_string};

pub fn handle(context: Context) -> Result<()> {
    if !exists(context.scripts_dir()).context("failed to check if scripts dir exists")? {
        create_dir_all(context.scripts_dir()).context("failed to create scripts dir")?;
    }

    let terrain = Terrain::from_toml(read_to_string(context.toml_path()?).context(format!(
        "failed to read terrain.toml from path {:?}",
        context.toml_path()
    ))?)
    .expect("expected terrain to created from toml");

    context.shell().generate_scripts(&context, terrain)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::client::shell::Zsh;
    use crate::client::types::context::Context;
    use crate::client::utils::{
        AssertTerrain, ExpectShell, IN_CURRENT_DIR, WITH_EXAMPLE_TERRAIN_TOML,
    };
    use anyhow::Result;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    pub fn generates_script() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for("example_biome", central_dir.path())
            .compile_terrain_script_for("none", central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
        fs::copy("./tests/data/terrain.example.toml", terrain_toml)
            .expect("test file to be copied");

        let central_dir1 = central_dir.path();
        let script_dir = central_dir1.join("scripts");
        fs::create_dir_all(script_dir).expect("test scripts dir to be created");

        super::handle(context).expect("no error to be thrown");

        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
            .was_initialized(IN_CURRENT_DIR, WITH_EXAMPLE_TERRAIN_TOML)
            .script_was_created_for("none")
            .script_was_created_for("example_biome");

        Ok(())
    }

    #[test]
    pub fn creates_scripts_dir_if_necessary() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let expected_shell_operation = ExpectShell::to()
            .compile_terrain_script_for("example_biome", central_dir.path())
            .compile_terrain_script_for("none", central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            Zsh::build(expected_shell_operation),
        );

        let terrain_toml: PathBuf = current_dir.path().join("terrain.toml");
        fs::copy("./tests/data/terrain.example.toml", terrain_toml)
            .expect("test file to be copied");

        super::handle(context).expect("no error to be thrown");

        // assert example_biome script is created
        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
            .was_initialized(IN_CURRENT_DIR, WITH_EXAMPLE_TERRAIN_TOML)
            .script_was_created_for("none")
            .script_was_created_for("example_biome");

        Ok(())
    }
}
