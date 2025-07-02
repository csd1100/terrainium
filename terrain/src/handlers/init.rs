use std::fs::{self, File};

use anyhow::{Context as _, Result};

use crate::context::Context;
use crate::types::terrain::Terrain;

/// Creates `terrain.toml` for the current directory.
///
/// If `central` flag was passed the `terrain.toml` will be created in central
/// directory.
/// If `example` flag is passed example [Terrain] will be created.
/// If `edit` is passed editor will be launched.
pub fn handle(context: Context, example: bool, _edit: bool) -> Result<()> {
    if !fs::exists(context.scripts_dir()).context("failed to check if scripts dir exists")? {
        fs::create_dir_all(context.scripts_dir()).context("failed to create scripts dir")?;
    }

    let toml_path = context.toml_path();

    let mut _file = File::create_new(toml_path).context("error while creating new terrain.toml")?;

    let terrain = if example {
        Terrain::example()
    } else {
        Terrain::default()
    };

    let _toml_str = terrain
        .to_toml(context.terrain_dir())
        .expect("default or example terrain to be parsed to toml");

    // file.write(toml_str.as_ref())
    //     .context("failed to write terrain in toml file")?;
    //
    // if edit {
    //     edit::run_editor(context.executor(), toml_path, context.terrain_dir())?;
    //     // get updated terrain after edit
    //     (terrain, _) = Terrain::get_validated_and_fixed_terrain(&context)?;
    // }
    //
    // context.shell().generate_scripts(&context, terrain)?;
    //
    Ok(())
}
