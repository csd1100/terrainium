use std::collections::BTreeMap;
use std::env::VarError;
use std::path::Path;

use pretty_assertions::assert_eq;

use crate::client::types::commands::Commands;
use crate::common::types::command::Command;

pub mod assertions;
pub mod constants;

/// # Safety
///
/// Setting or removing env variable can affect the other threads
/// in program so tests cannot be run in the parallel. Always use
/// this method in test annotated with #\[serial]
pub unsafe fn set_env_var(key: &str, value: Option<&str>) -> Result<String, VarError> {
    let orig_env = std::env::var(key);
    if let Some(val) = value {
        unsafe { std::env::set_var(key, val) };
    } else {
        unsafe { std::env::remove_var(key) };
    }

    orig_env
}

/// # Safety
///
/// Setting or removing env variable can affect the other threads
/// in program so tests cannot be run in the parallel. Always use
/// this method in test annotated with #\[serial]
pub unsafe fn restore_env_var(key: &str, orig_env: anyhow::Result<String, VarError>) {
    // FIX: the tests run in parallel so restoring env vars won't help if vars have same key
    if let Ok(orig_var) = orig_env {
        unsafe { std::env::set_var(key, &orig_var) };
        assert!(std::env::var(key).is_ok());
        assert_eq!(orig_var, std::env::var(key).expect("var to be present"));
    } else {
        unsafe { std::env::remove_var(key) };
        assert!(std::env::var(key).is_err());
    }
}

pub(crate) fn expected_aliases_example_biome() -> BTreeMap<String, String> {
    let mut expected_aliases: BTreeMap<String, String> = BTreeMap::new();
    expected_aliases.insert(
        "tenter".to_string(),
        "terrain enter --biome example_biome".to_string(),
    );
    expected_aliases.insert("texit".to_string(), "terrain exit".to_string());
    expected_aliases
}

pub(crate) fn expected_constructor_foreground_example_biome(terrain_dir: &Path) -> Vec<Command> {
    vec![
        Command::new(
            "/bin/echo".to_string(),
            vec!["entering terrain".to_string()],
            Some(terrain_dir.to_path_buf()),
        ),
        Command::new(
            "/bin/echo".to_string(),
            vec!["entering biome example_biome".to_string()],
            Some(terrain_dir.to_path_buf()),
        ),
    ]
}

pub(crate) fn expected_constructor_background_example_biome(terrain_dir: &Path) -> Vec<Command> {
    vec![Command::new(
        "/bin/bash".to_string(),
        vec![
            "-c".to_string(),
            "${PWD}/tests/scripts/print_num_for_10_sec".to_string(),
        ],
        Some(terrain_dir.to_path_buf()),
    )]
}

pub(crate) fn expected_destructor_foreground_example_biome(terrain_dir: &Path) -> Vec<Command> {
    vec![
        Command::new(
            "/bin/echo".to_string(),
            vec!["exiting terrain".to_string()],
            Some(terrain_dir.to_path_buf()),
        ),
        Command::new(
            "/bin/echo".to_string(),
            vec!["exiting biome example_biome".to_string()],
            Some(terrain_dir.to_path_buf()),
        ),
    ]
}

pub(crate) fn expected_destructor_background_example_biome(terrain_dir: &Path) -> Vec<Command> {
    vec![Command::new(
        "/bin/bash".to_string(),
        vec![
            "-c".to_string(),
            "${TERRAIN_DIR}/tests/scripts/print_num_for_10_sec".to_string(),
        ],
        Some(terrain_dir.to_path_buf()),
    )]
}

pub(crate) fn expected_constructors_example_biome(terrain_dir: &Path) -> Commands {
    Commands::new(
        expected_constructor_foreground_example_biome(terrain_dir),
        expected_constructor_background_example_biome(terrain_dir),
    )
}

pub(crate) fn expected_destructors_example_biome(terrain_dir: &Path) -> Commands {
    Commands::new(
        expected_destructor_foreground_example_biome(terrain_dir),
        expected_destructor_background_example_biome(terrain_dir),
    )
}
