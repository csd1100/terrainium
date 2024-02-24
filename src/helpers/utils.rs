#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub mod fs {
    use anyhow::Result;
    use std::{
        fs::File,
        path::{Path, PathBuf},
    };

    pub fn get_cwd() -> Result<PathBuf> {
        Ok(std::env::current_dir()?)
    }

    pub fn write_file(path: &Path, contents: String) -> Result<()> {
        Ok(std::fs::write(path, contents)?)
    }

    pub fn copy_file(from: &Path, to: &Path) -> Result<u64> {
        Ok(std::fs::copy(from, to)?)
    }

    pub fn create_dir_if_not_exist(dir: &Path) -> Result<bool> {
        if !Path::try_exists(dir)? {
            println!("creating a directory at path {}", dir.to_string_lossy());
            std::fs::create_dir_all(dir)?;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn get_process_log_file(session_id: &String, filename: String) -> Result<(PathBuf, File)> {
        let tmp = PathBuf::from(format!("/tmp/terrainium-{}", session_id));
        create_dir_if_not_exist(&tmp)?;

        let mut out_path = tmp.clone();
        out_path.push(filename);
        let out = File::options()
            .append(true)
            .create_new(true)
            .open(&out_path)?;

        Ok((out_path, out))
    }

    pub fn remove_all_script_files(central_store: &Path) -> Result<()> {
        if let std::result::Result::Ok(entries) = std::fs::read_dir(central_store) {
            for entry in entries {
                let std::result::Result::Ok(entry) = entry else {
                    continue;
                };
                if let Some(ext) = entry.path().extension() {
                    if ext.to_str() == Some("zwc") || ext.to_str() == Some("zsh") {
                        std::fs::remove_file(entry.path())?;
                    }
                };
            }
        }
        Ok(())
    }
}

#[cfg_attr(test, automock)]
pub mod misc {
    use uuid::Uuid;

    pub fn get_uuid() -> String {
        Uuid::new_v4().to_string()
    }
}
