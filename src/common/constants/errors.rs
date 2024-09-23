use thiserror::Error;

#[derive(Error, Debug)]
pub enum TerrainiumErrors {
    #[error("")]
    InvalidBiome(String),
}
