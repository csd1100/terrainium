use thiserror::Error;

#[derive(Error, Debug)]
pub enum TerrainiumErrors {
    #[error("biome `{0}` is not present")]
    BiomeNotFound(String),

    #[error("biome `{0}` already exists")]
    BiomeAlreadyExists(String),

    #[error("biomes are not defined")]
    BiomesNotDefined,

    #[error("default biome is not defined")]
    DefaultBiomeNotDefined,

    #[error("aliases are not defined")]
    AliasesNotDefined,

    #[error("environment variables are not defined")]
    EnvsNotDefined,

    #[error("home directory path not found")]
    UnableToFindHome,

    #[error("invalid home directory found")]
    InvalidHomeDirectory,
}
