use std::collections::BTreeMap;
use std::path::Path;

use crate::constants::{
    TERRAIN_AUTO_APPLY, TERRAIN_DIR, TERRAIN_NAME, TERRAIN_SELECTED_BIOME, TERRAIN_SESSION_ID,
};
use crate::types::biome::Biome;
use crate::types::terrain::AutoApply;

pub struct Environment {
    /// name of the terrain
    name: String,
    /// default biome of the terrain
    default_biome: Option<String>,
    /// biome selected to create this environment
    selected_biome: String,
    /// auto apply value of the terrain
    auto_apply: AutoApply,
    /// environment after merging
    merged: Biome,
}

impl Environment {
    pub fn new(
        name: String,
        default_biome: Option<String>,
        selected_biome: String,
        auto_apply: AutoApply,
        merged: Biome,
    ) -> Self {
        Self {
            name,
            default_biome,
            selected_biome,
            auto_apply,
            merged,
        }
    }

    /// environment variables set in shell by terrainium to operate
    pub fn terrainium_vars(
        &self,
        session_id: String,
        terrain_dir: &Path,
        is_auto_apply: bool,
    ) -> BTreeMap<String, String> {
        let mut envs = BTreeMap::new();
        envs.insert(TERRAIN_NAME.to_string(), self.name.clone());
        envs.insert(TERRAIN_SESSION_ID.to_string(), session_id);
        envs.insert(
            TERRAIN_SELECTED_BIOME.to_string(),
            self.merged.name().to_string(),
        );
        envs.insert(TERRAIN_DIR.to_string(), terrain_dir.display().to_string());
        if is_auto_apply {
            envs.insert(TERRAIN_AUTO_APPLY.to_string(), self.auto_apply.to_string());
        }
        envs
    }
}
