use crate::client::test_utils::{expected_env_vars_example_biome, expected_env_vars_none};
use crate::client::types::terrain::AutoApply;
use crate::common::constants::{
    EXAMPLE_BIOME, FPATH, NONE, TERRAIN_AUTO_APPLY, TERRAIN_DIR, TERRAIN_ENABLED, TERRAIN_INIT_FN,
    TERRAIN_INIT_SCRIPT, TERRAIN_SESSION_ID, TERRAIN_TOML, TRUE,
};
use crate::common::types::pb;
use std::collections::BTreeMap;
use std::path::Path;

pub const TEST_TERRAIN_NAME: &str = "terrainium";
pub const TEST_TIMESTAMP: &str = "timestamp";
pub const TEST_SESSION_ID: &str = "session_id";
pub const TEST_FPATH: &str = "/usr/share/zsh/completions";
pub const TEST_TERRAIN_DIR: &str = "/tmp/terrain_dir";
pub const TEST_CENTRAL_DIR: &str = "/tmp/central_dir";

pub fn expected_envs_with_activate_example_biome(
    is_auto_apply: bool,
    auto_apply: &AutoApply,
) -> BTreeMap<String, String> {
    let script = format!("{TEST_CENTRAL_DIR}/scripts/terrain-example_biome.zwc");

    let mut envs = expected_env_vars_example_biome(Path::new(TEST_TERRAIN_DIR));
    envs.insert(FPATH.to_string(), format!("{}:{}", script, TEST_FPATH));
    envs.insert(
        TERRAIN_INIT_FN.to_string(),
        "terrain-example_biome.zsh".to_string(),
    );
    envs.insert(TERRAIN_INIT_SCRIPT.to_string(), script);
    envs.insert(TERRAIN_DIR.to_string(), TEST_TERRAIN_DIR.to_string());
    envs.insert(TERRAIN_ENABLED.to_string(), TRUE.to_string());
    envs.insert(TERRAIN_SESSION_ID.to_string(), TEST_SESSION_ID.to_string());
    if is_auto_apply {
        envs.insert(TERRAIN_AUTO_APPLY.to_string(), auto_apply.into());
    }
    envs
}

pub fn expected_activate_request_example_biome(
    is_background: bool,
    is_auto_apply: bool,
    auto_apply: &AutoApply,
) -> pb::Activate {
    let terrain_dir = TEST_TERRAIN_DIR.to_string();
    let toml_path = format!("{terrain_dir}/{TERRAIN_TOML}");
    pb::Activate {
        session_id: TEST_SESSION_ID.to_string(),
        terrain_name: TEST_TERRAIN_NAME.to_string(),
        biome_name: EXAMPLE_BIOME.to_string(),
        terrain_dir: terrain_dir.clone(),
        toml_path: toml_path.clone(),
        start_timestamp: TEST_TIMESTAMP.to_string(),
        is_background,
        constructors: if is_background {
            Some(pb::Execute {
                session_id: Some(TEST_SESSION_ID.to_string()),
                terrain_name: TEST_TERRAIN_NAME.to_string(),
                biome_name: EXAMPLE_BIOME.to_string(),
                terrain_dir: terrain_dir.clone(),
                toml_path,
                is_constructor: true,
                timestamp: TEST_TIMESTAMP.to_string(),
                commands: vec![pb::Command {
                    exe: "/bin/bash".to_string(),
                    args: vec![
                        "-c".to_string(),
                        "$PWD/tests/scripts/print_num_for_10_sec".to_string(),
                    ],
                    envs: expected_envs_with_activate_example_biome(is_auto_apply, auto_apply),
                    cwd: terrain_dir,
                }],
            })
        } else {
            None
        },
    }
}

pub fn expected_envs_with_activate_none(
    is_auto_apply: bool,
    auto_apply: &AutoApply,
) -> BTreeMap<String, String> {
    let script = format!("{TEST_CENTRAL_DIR}/scripts/terrain-none.zwc");

    let mut envs = expected_env_vars_none(Path::new(TEST_TERRAIN_DIR));
    envs.insert(FPATH.to_string(), format!("{}:{}", script, TEST_FPATH));
    envs.insert(TERRAIN_INIT_FN.to_string(), "terrain-none.zsh".to_string());
    envs.insert(TERRAIN_INIT_SCRIPT.to_string(), script);
    envs.insert(TERRAIN_DIR.to_string(), TEST_TERRAIN_DIR.to_string());
    envs.insert(TERRAIN_ENABLED.to_string(), TRUE.to_string());
    envs.insert(TERRAIN_SESSION_ID.to_string(), TEST_SESSION_ID.to_string());
    if is_auto_apply {
        envs.insert(TERRAIN_AUTO_APPLY.to_string(), auto_apply.into());
    }
    envs
}

pub fn expected_activate_request_none(is_background: bool) -> pb::Activate {
    let terrain_dir = TEST_TERRAIN_DIR.to_string();
    let toml_path = format!("{terrain_dir}/{TERRAIN_TOML}");

    pb::Activate {
        session_id: TEST_SESSION_ID.to_string(),
        terrain_name: TEST_TERRAIN_NAME.to_string(),
        biome_name: NONE.to_string(),
        terrain_dir: terrain_dir.clone(),
        toml_path: toml_path.clone(),
        start_timestamp: TEST_TIMESTAMP.to_string(),
        is_background,
        constructors: None,
    }
}
