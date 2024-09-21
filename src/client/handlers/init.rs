use crate::client::types::context::Context;
use anyhow::{Context as AnyhowContext, Result};
use std::fs::File;

pub fn handle(context: Context) -> Result<()> {
    File::create_new(context.get_toml_path()).context("terrain for this project is already present. edit existing terrain with `terrain edit` command")?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::client::types::context::Context;
    use anyhow::Result;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn init_creates_terrain_toml_in_current_dir() -> Result<()> {
        let current_dir = tempdir()?;

        let context: Context = Context::build(current_dir.path().into());

        super::handle(context)?;

        let mut terrain_toml_path = current_dir.into_path();
        terrain_toml_path.push("terrain.toml");

        assert!(fs::exists(terrain_toml_path)?, "expected terrain.toml to be created");

        Ok(())
    }

    #[test]
    fn init_throws_error_if_terrain_toml_exists_in_current_dir() -> Result<()> {
        let current_dir = tempdir()?;

        let context: Context = Context::build(current_dir.path().into());

        let mut terrain_toml_path = current_dir.into_path();
        terrain_toml_path.push("terrain.toml");

        fs::write(terrain_toml_path, "")?;

        let err = super::handle(context).err().expect("expected error to be thrown");

        assert_eq!(err.to_string(), "terrain for this project is already present. edit existing terrain with `terrain edit` command");

        Ok(())
    }
}