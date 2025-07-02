use std::collections::BTreeMap;

use crate::constants::{EDITOR, ENV_VAR, NESTED_POINTER, NULL_POINTER, PAGER, POINTER_ENV_VAR};

pub mod test_zsh;

pub const TEST_SESSION_ID: &str = "session-id";

pub fn expected_env_vars_example_biome() -> BTreeMap<String, String> {
    let mut expected_envs = BTreeMap::new();
    expected_envs.insert(EDITOR.to_string(), "nvim".to_string());
    expected_envs.insert(NULL_POINTER.to_string(), "${NULL}".to_string());
    expected_envs.insert(PAGER.to_string(), "less".to_string());
    expected_envs.insert(ENV_VAR.to_string(), "overridden_env_val".to_string());
    expected_envs.insert(
        NESTED_POINTER.to_string(),
        "overridden_env_val-overridden_env_val-${NULL}".to_string(),
    );
    expected_envs.insert(
        POINTER_ENV_VAR.to_string(),
        "overridden_env_val".to_string(),
    );
    expected_envs
}
