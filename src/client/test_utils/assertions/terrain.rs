use std::fs::read_to_string;
use std::path::Path;

pub struct AssertTerrain<'a> {
    current_dir: &'a Path,
    central_dir: &'a Path,
    older_toml_path: &'a str,
    older_toml: String,
}

impl<'a> AssertTerrain<'a> {
    pub fn with_dirs(current_dir: &'a Path, central_dir: &'a Path) -> Self {
        Self {
            current_dir,
            central_dir,
            older_toml_path: "",
            older_toml: "".to_string(),
        }
    }

    pub fn with_dirs_and_existing(
        current_dir: &'a Path,
        central_dir: &'a Path,
        existing_terrain: &'a str,
    ) -> Self {
        let older_toml = read_to_string(existing_terrain).expect("to be read");
        Self {
            current_dir,
            central_dir,
            older_toml_path: existing_terrain,
            older_toml,
        }
    }

    pub fn scripts_dir_was_created(self) -> Self {
        assert!(
            self.central_dir.join("scripts").exists(),
            "failed to find scripts dir"
        );
        self
    }

    pub fn central_dir_is_created(self) -> Self {
        assert!(self.central_dir.exists(), "failed to find central dir");
        self
    }

    pub fn was_initialized(self, in_central: bool, mode: &'static str) -> Self {
        let toml = if in_central {
            self.central_dir
        } else {
            self.current_dir
        }
        .join("terrain.toml");

        assert!(toml.exists(), "failed to find terrain.toml");
        assert_eq!(
            read_to_string(&toml).expect("to find terrain.toml"),
            read_to_string(mode).expect("to find test terrain.toml"),
            "failed to validate that terrain.toml was created for in_central {in_central} for test terrain: {mode}",
        );

        self
    }

    pub fn script_was_created_for(self, biome_name: &str) -> Self {
        let script = self
            .central_dir
            .join("scripts")
            .join(format!("terrain-{biome_name}.zsh"));

        assert!(
            script.exists(),
            "failed to find script for biome {biome_name}",
        );

        self
    }

    pub fn was_updated(self, in_central: bool, new_toml_path: &'static str) -> Self {
        let toml = if in_central {
            self.central_dir
        } else {
            self.current_dir
        }
        .join("terrain.toml");

        assert!(toml.exists(), "failed to find terrain.toml");

        let new_toml_contents = read_to_string(&toml).expect("to find terrain.toml");

        assert_ne!(self.older_toml_path, new_toml_path);
        assert_ne!(self.older_toml, new_toml_contents);
        assert_eq!(
            new_toml_contents,
            read_to_string(new_toml_path).expect("to find test terrain.toml"),
            "failed to validate terrain.toml was created for in_central {in_central} for test terrain: {new_toml_path}",
        );

        self
    }

    pub fn with_backup(self, in_central: bool) -> Self {
        let backup = if in_central {
            self.central_dir
        } else {
            self.current_dir
        }
        .join("terrain.toml.bkp");

        assert!(backup.exists(), "failed to find terrain.toml");
        assert_eq!(
            self.older_toml,
            read_to_string(backup).expect("to find test terrain.toml"),
            "failed to check terrain.toml.bkp was created for in_central {in_central} for test terrain: {}",
            self.older_toml_path
        );

        self
    }

    pub fn was_not_updated(self, in_central: bool) -> Self {
        let toml = if in_central {
            self.central_dir
        } else {
            self.current_dir
        }
        .join("terrain.toml");

        assert!(toml.exists(), "failed to find terrain.toml");

        let new_toml_contents = read_to_string(&toml).expect("to find terrain.toml");

        assert_eq!(
            new_toml_contents,
            read_to_string(self.older_toml_path).expect("to find test terrain.toml"),
            "failed to check terrain.toml was created for in_central {in_central} for test terrain: {}",
            self.older_toml_path
        );

        self
    }
}
