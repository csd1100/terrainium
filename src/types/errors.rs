use thiserror::Error;

#[derive(Error, Debug)]
pub enum TerrainiumErrors {
    #[error("biome `{0}` not found")]
    BiomeNotFound(String),

    #[error("home directory path not found")]
    UnableToFindHome,

    #[error("invalid home directory found")]
    InvalidHomeDirectory,
}
