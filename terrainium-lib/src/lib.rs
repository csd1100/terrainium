pub mod command;
pub mod constants;
pub mod executor;
pub mod paths;
pub mod socket;
pub mod styles;
#[cfg(any(test, feature = "test-exports"))]
pub mod test_utils;
pub mod version;

// FIXME: remove clippy allow in future when prost has update
#[allow(clippy::large_enum_variant)]
pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/terrainium.v1.rs"));
}
