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
    use crate::client::test_utils::assertions::terrain::AssertTerrain;
    use crate::client::test_utils::assertions::zsh::ExpectZSH;
    use crate::client::types::config::Config;
    use crate::client::types::context::Context;
    use crate::client::types::terrain::Terrain;
    use crate::common::constants::{EXAMPLE_BIOME, NONE};
    use crate::common::execute::MockExecutor;
    use anyhow::Result;
    use tempfile::tempdir;

    #[test]
    fn generates_script() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let executor = ExpectZSH::with(MockExecutor::new(), current_dir.path().to_path_buf())
            .compile_terrain_script_for(EXAMPLE_BIOME, central_dir.path())
            .compile_terrain_script_for(NONE, central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Config::default(),
            executor,
        );

        super::handle(context, Terrain::example()).expect("no error to be thrown");

        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
            .script_was_created_for(NONE)
            .script_was_created_for(EXAMPLE_BIOME);

        Ok(())
    }

    #[test]
    fn creates_scripts_dir_if_necessary() -> Result<()> {
        let current_dir = tempdir()?;
        let central_dir = tempdir()?;

        let executor = ExpectZSH::with(MockExecutor::new(), current_dir.path().to_path_buf())
            .compile_terrain_script_for(EXAMPLE_BIOME, central_dir.path())
            .compile_terrain_script_for(NONE, central_dir.path())
            .successfully();

        let context: Context = Context::build(
            current_dir.path().into(),
            central_dir.path().into(),
            current_dir.path().join("terrain.toml"),
            Config::default(),
            executor,
        );

        super::handle(context, Terrain::example()).expect("no error to be thrown");

        // assert example_biome script is created
        AssertTerrain::with_dirs(current_dir.path(), central_dir.path())
            .script_was_created_for(NONE)
            .script_was_created_for(EXAMPLE_BIOME);

        Ok(())
    }
}
