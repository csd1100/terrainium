#[cfg(feature = "terrain-schema")]
use anyhow::Result;

#[cfg(feature = "terrain-schema")]
pub fn handle() -> Result<()> {
    inner::generate_and_store_schema()
}

#[cfg(feature = "terrain-schema")]
mod inner {
    use std::fs;
    use std::path::PathBuf;

    use crate::client::types::config::Config;
    use crate::client::types::terrain::Terrain;
    use crate::daemon::types::config::DaemonConfig;
    use anyhow::Result;
    use schemars::schema_for;

    pub fn generate_and_store_schema() -> Result<()> {
        let terrain_toml_schema = schema_for!(Terrain);
        let terrain_toml_schema =
            serde_json::to_string_pretty(&terrain_toml_schema).expect("schema to be generated");
        fs::write(
            PathBuf::from("./schema/terrain-schema.json"),
            terrain_toml_schema,
        )?;
        let terrainiumd_conf_schema = schema_for!(DaemonConfig);
        let terrainiumd_conf_schema =
            serde_json::to_string_pretty(&terrainiumd_conf_schema).expect("schema to be generated");
        fs::write(
            PathBuf::from("./schema/terrainiumd-conf-schema.json"),
            terrainiumd_conf_schema,
        )?;
        let terrainium_conf_schema = schema_for!(Config);
        let terrainium_conf_schema =
            serde_json::to_string_pretty(&terrainium_conf_schema).expect("schema to be generated");
        fs::write(
            PathBuf::from("./schema/terrainium-conf-schema.json"),
            terrainium_conf_schema,
        )?;
        Ok(())
    }
}
