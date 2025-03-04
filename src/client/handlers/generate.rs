use crate::client::shell::Shell;
use crate::client::types::context::Context;
use crate::client::types::terrain::Terrain;
use anyhow::{Context as AnyhowContext, Result};
use std::fs::{create_dir_all, exists};

pub fn handle(context: Context, terrain: Terrain) -> Result<()> {
    if !exists(context.scripts_dir()).context("failed to check if scripts dir exists")? {
        create_dir_all(context.scripts_dir()).context("failed to create scripts dir")?;
    }

    context.shell().generate_scripts(&context, terrain)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::client::shell::Zsh;
    use crate::client::types::context::Context;
    use crate::client::types::terrain::Terrain;
    use crate::client::utils::{AssertTerrain, ExpectShell};
    use anyhow::Result;
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
            current_dir.path().join("terrain.toml"),
            Zsh::build(expected_shell_operation),
        );

        super::handle(context, Terrain::example()).expect("no error to be thrown");

        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
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
            current_dir.path().join("terrain.toml"),
            Zsh::build(expected_shell_operation),
        );

        super::handle(context, Terrain::example()).expect("no error to be thrown");

        // assert example_biome script is created
        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
            .script_was_created_for("none")
            .script_was_created_for("example_biome");

        Ok(())
    }
}
