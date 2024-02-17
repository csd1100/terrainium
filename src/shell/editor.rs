#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub mod edit {
    use std::path::Path;

    use anyhow::{Context, Result};
    use mockall_double::double;

    #[double]
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

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use anyhow::Result;
    use serial_test::serial;

    use crate::shell::execute::mock_spawn;

    #[test]
    #[serial]
    fn start_editor_if_env_var() -> Result<()> {
        // setup
        let real_editor = std::env::var("EDITOR").ok();

        std::env::set_var("EDITOR", "value");

        let exp_args = vec!["file.txt"];
        let mock_spawn_editor = mock_spawn::and_wait_context();
        mock_spawn_editor
            .expect()
            .withf(move |exe, args, envs| exe == "value" && *args == exp_args && envs.is_none())
            .return_once(|_, _, _| Ok(()))
            .times(1);

        super::edit::file(&PathBuf::from("file.txt"))?;

        // cleanup
        if let Some(editor) = real_editor {
            std::env::set_var("EDITOR", editor)
        }

        Ok(())
    }

    #[test]
    #[serial]
    fn return_err_if_no_editor() -> Result<()> {
        // setup
        let real_editor = std::env::var("EDITOR").ok();

        std::env::remove_var("EDITOR");

        let actual = super::edit::file(&PathBuf::from("file.txt"))
            .unwrap_err()
            .to_string();

        assert_eq!(
            String::from("environment variable EDITOR not defined to edit terrain."),
            actual
        );

        // cleanup
        if let Some(editor) = real_editor {
            std::env::set_var("EDITOR", editor)
        }

        Ok(())
    }
}
