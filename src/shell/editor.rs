#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub mod edit {
    use std::path::Path;

    use anyhow::{Context, Result};

    use crate::shell::execute::spawn;

    pub fn file(file: &Path) -> Result<()> {
        let editor = std::env::var("EDITOR")
            .context("environment variable EDITOR not defined to edit terrain.")?;

        let file = file.to_str().expect("filepath to be converted to string");

        spawn::and_wait(&editor, vec![file], None)
            .context(format!("failed to start editor {}", editor))?;

        Ok(())
    }
}
