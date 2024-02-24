#[cfg(feature = "terrain-schema")]
use anyhow::Result;

#[cfg(feature = "terrain-schema")]
pub fn handle() -> Result<()> {
    inner::generate_and_store_schema()
}

#[cfg(feature = "terrain-schema")]
mod inner {
    use std::path::PathBuf;

    use crate::{helpers::operations::write_file, types::terrain::Terrain};
    use anyhow::Result;
    use schemars::schema_for;

    pub fn generate_and_store_schema() -> Result<()> {
        let schema = schema_for!(Terrain);
        let json = serde_json::to_string_pretty(&schema).unwrap();
        write_file(&PathBuf::from("./schema/terrain-schema.json"), json)?;
        Ok(())
    }
}
