use serde::Serialize;

use super::biomes::Biome;

#[derive(Debug, Serialize, PartialEq)]
pub struct PrintableTerrain {
    pub all: bool,
    pub default_biome: Option<String>,
    pub selected_biome: Option<String>,
    pub biome: Biome,
}
