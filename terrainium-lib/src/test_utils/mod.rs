use std::env::VarError;

pub mod execute;

pub use crate::executor::MockExecute;

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
