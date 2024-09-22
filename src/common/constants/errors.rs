use thiserror::Error;

#[derive(Error, Debug)]
pub enum TerrainiumErrors {
    #[error("the biome {0} does not exists")]
    InvalidBiome(String),

    #[error("terrain for this project is already present. edit existing terrain with `terrain edit` command"
    )]
    AlreadyExists,
}
