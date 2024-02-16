use std::collections::HashMap;

use anyhow::Result;
#[cfg(test)]
use mockall::automock;

use crate::types::args::BiomeArg;

pub fn handle(biome: Option<BiomeArg>, envs: Option<&HashMap<String, String>>) -> Result<()> {
    run::constructors(biome, envs)
}

#[cfg_attr(test, automock)]
pub mod run {

    use anyhow::{Context, Result};
    use mockall_double::double;
    use std::collections::HashMap;

    use crate::{shell::background::start_background_processes, types::args::BiomeArg};

    #[double]
    use crate::helpers::helpers::fs;

    pub fn constructors<'a>(
        biome: Option<BiomeArg>,
        envs: Option<&'a HashMap<String, String>>,
    ) -> Result<()> {
        let terrain = fs::get_parsed_terrain()?
            .get(biome)
            .context("unable to select biome to call constructors")?;
        if let Some(envs) = envs {
            return start_background_processes(terrain.constructors, envs)
                .context("unable to start background processes");
        }
        start_background_processes(terrain.constructors, &terrain.env.unwrap_or_default())
            .context("unable to start background processes")
    }
}
