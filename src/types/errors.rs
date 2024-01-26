use thiserror::Error;

#[derive(Error, Debug)]
pub enum TerrainiumErrors {
    #[error("biome `{0}` not found")]
    BiomeNotFound(String),
}
