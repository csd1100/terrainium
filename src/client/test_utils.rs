use std::env::VarError;
pub mod assertions;
pub mod constants;

pub fn set_env_var(key: String, value: Option<String>) -> Result<String, VarError> {
    // FIX: the tests run in parallel so setting same env var will cause tests to fail
    // as env var is not reset yet
    let orig_env = std::env::var(&key);
    if let Some(val) = value {
        std::env::set_var(&key, val);
    } else {
        std::env::remove_var(&key);
    }

    orig_env
}

pub fn restore_env_var(key: String, orig_env: anyhow::Result<String, VarError>) {
    // FIX: the tests run in parallel so restoring env vars won't help if vars have same key
    if let Ok(orig_var) = orig_env {
        std::env::set_var(&key, &orig_var);
        assert!(std::env::var(&key).is_ok());
        assert_eq!(orig_var, std::env::var(&key).expect("var to be present"));
    } else {
        std::env::remove_var(&key);
        assert!(std::env::var(&key).is_err());
    }
}
