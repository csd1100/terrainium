#[cfg(test)]
pub(crate) mod test {
    use std::path::{Path, PathBuf};

    pub(crate) fn script_path(central_dir: &Path, biome_name: &String) -> PathBuf {
        scripts_dir(central_dir)
            .clone()
            .join(format!("terrain-{}.zsh", biome_name))
    }

    pub(crate) fn scripts_dir(central_dir: &Path) -> PathBuf {
        central_dir.join("scripts")
    }
}
