use std::collections::BTreeMap;
use std::path::Path;

use crate::client::test_utils::{
    expected_constructor_background_example_biome, expected_destructor_background_example_biome,
};
use crate::client::types::terrain::AutoApply;
use crate::common::constants::{
    EXAMPLE_BIOME, FPATH, TERRAIN_AUTO_APPLY, TERRAIN_DIR, TERRAIN_INIT_FN, TERRAIN_INIT_SCRIPT,
    TERRAIN_NAME, TERRAIN_SELECTED_BIOME, TERRAIN_SESSION_ID, TERRAIN_TOML, TEST_TIMESTAMP,
};
use crate::common::types::pb;

pub const TEST_TERRAIN_NAME: &str = "terrainium";
pub const TEST_TIMESTAMP_NUMERIC: &str = "19700101000000";
pub const TEST_SESSION_ID: &str = "session_id";
pub const TEST_FPATH: &str = "/usr/share/zsh/completions";
pub const TEST_TERRAIN_DIR: &str = "/tmp/terrain_dir";
pub const TEST_CENTRAL_DIR: &str = "/tmp/central_dir";
pub const TEST_DIRECTORY: &str = "/tmp/terrainium-testing-46678f282cf1/";

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
                envs: expected_envs_with_activate_example_biome(is_auto_apply, auto_apply),
                commands: vec![pb::Command {
                    exe: "/bin/bash".to_string(),
                    args: vec![
                        "-c".to_string(),
                        "${PWD}/tests/scripts/print_num_for_10_sec".to_string(),
                    ],
                    cwd: terrain_dir,
                }],
            })
        } else {
            None
        },
    }
}

pub(crate) fn expected_execute_request_example_biome(
    session_id: Option<String>,
    is_constructor: bool,
) -> pb::Execute {
    let terrain_dir = TEST_TERRAIN_DIR.to_string();
    let toml_path = format!("{terrain_dir}/{TERRAIN_TOML}");
    let commands = if is_constructor {
        expected_constructor_background_example_biome(Path::new(TEST_TERRAIN_DIR))
    } else {
        expected_destructor_background_example_biome(Path::new(TEST_TERRAIN_DIR))
    };
    let commands = commands.into_iter().map(|cmd| cmd.into()).collect();

    pb::Execute {
        session_id,
        terrain_name: TEST_TERRAIN_NAME.to_string(),
        biome_name: EXAMPLE_BIOME.to_string(),
        terrain_dir: terrain_dir.clone(),
        toml_path,
        is_constructor,
        timestamp: TEST_TIMESTAMP.to_string(),
        envs: expected_env_vars_example_biome(),
        commands,
    }
}

pub fn expected_deactivate_request_example_biome(session_id: &str) -> pb::Deactivate {
    pb::Deactivate {
        session_id: session_id.to_string(),
        terrain_name: TEST_TERRAIN_NAME.to_string(),
        end_timestamp: TEST_TIMESTAMP.to_string(),
        destructors: Some(expected_execute_request_example_biome(
            Some(session_id.to_string()),
            false,
        )),
    }
}

pub enum RequestFor {
    SessionId(String),
    Recent(u32),
    None,
}

pub fn expected_status_request(
    request_for: RequestFor,
    current_session_id: &str,
) -> pb::StatusRequest {
    pb::StatusRequest {
        terrain_name: TEST_TERRAIN_NAME.to_string(),
        identifier: {
            let id = match request_for {
                RequestFor::SessionId(session_id) => {
                    pb::status_request::Identifier::SessionId(session_id)
                }
                RequestFor::Recent(r) => pb::status_request::Identifier::Recent(r),
                RequestFor::None => {
                    if current_session_id.is_empty() {
                        pb::status_request::Identifier::Recent(0)
                    } else {
                        pb::status_request::Identifier::SessionId(current_session_id.to_string())
                    }
                }
            };
            Some(id)
        },
    }
}

pub(crate) fn expected_zsh_env_vars(biome: &str) -> BTreeMap<String, String> {
    let script = format!("{TEST_CENTRAL_DIR}/scripts/terrain-{biome}.zwc");
    let mut envs = BTreeMap::new();
    envs.insert(FPATH.to_string(), format!("{}:{}", script, TEST_FPATH));
    envs.insert(TERRAIN_INIT_FN.to_string(), format!("terrain-{biome}.zsh"));
    envs.insert(TERRAIN_INIT_SCRIPT.to_string(), script);
    envs
}

pub(crate) fn expected_activation_env_vars(
    biome: &str,
    is_auto_apply: bool,
    auto_apply: &AutoApply,
    terrain_dir: &str,
) -> BTreeMap<String, String> {
    let mut envs = BTreeMap::new();
    envs.insert(TERRAIN_NAME.to_string(), TEST_TERRAIN_NAME.to_string());
    envs.insert(TERRAIN_SESSION_ID.to_string(), TEST_SESSION_ID.to_string());
    envs.insert(TERRAIN_SELECTED_BIOME.to_string(), biome.to_string());
    envs.insert(TERRAIN_DIR.to_string(), terrain_dir.to_string());
    if is_auto_apply {
        envs.insert(TERRAIN_AUTO_APPLY.to_string(), auto_apply.to_string());
    }
    envs
}

pub(crate) fn expected_env_vars_none() -> BTreeMap<String, String> {
    let mut expected_envs = BTreeMap::new();
    expected_envs.insert("EDITOR".to_string(), "vim".to_string());
    expected_envs.insert("NULL_POINTER".to_string(), "${NULL}".to_string());
    expected_envs.insert("PAGER".to_string(), "less".to_string());
    expected_envs.insert("ENV_VAR".to_string(), "env_val".to_string());
    expected_envs.insert(
        "NESTED_POINTER".to_string(),
        "env_val-env_val-${NULL}".to_string(),
    );
    expected_envs.insert("POINTER_ENV_VAR".to_string(), "env_val".to_string());
    expected_envs
}

pub(crate) fn expected_env_vars_example_biome() -> BTreeMap<String, String> {
    let mut expected_envs = BTreeMap::new();
    expected_envs.insert("EDITOR".to_string(), "nvim".to_string());
    expected_envs.insert("NULL_POINTER".to_string(), "${NULL}".to_string());
    expected_envs.insert("PAGER".to_string(), "less".to_string());
    expected_envs.insert("ENV_VAR".to_string(), "overridden_env_val".to_string());
    expected_envs.insert(
        "NESTED_POINTER".to_string(),
        "overridden_env_val-overridden_env_val-${NULL}".to_string(),
    );
    expected_envs.insert(
        "POINTER_ENV_VAR".to_string(),
        "overridden_env_val".to_string(),
    );
    expected_envs
}

pub fn expected_envs_with_activate_example_biome(
    is_auto_apply: bool,
    auto_apply: &AutoApply,
) -> BTreeMap<String, String> {
    let mut envs = expected_env_vars_example_biome();
    envs.append(&mut expected_activation_env_vars(
        EXAMPLE_BIOME,
        is_auto_apply,
        auto_apply,
        TEST_TERRAIN_DIR,
    ));
    envs.append(&mut expected_zsh_env_vars(EXAMPLE_BIOME));
    envs
}
