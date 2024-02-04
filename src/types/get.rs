use serde::Serialize;

use super::biomes::Biome;

#[derive(Debug, Serialize)]
pub struct PrintableTerrain {
    // terrain: String,
    // #[serde(skip_serializing)]
    pub all: bool,

    pub default_biome: Option<String>,
    pub selected_biome: Option<String>,
    pub biome: Biome,
}
