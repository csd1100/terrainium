const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const GIT_HASH: &str = include_str!(concat!(env!("OUT_DIR"), "/git_hash.txt"));
const BUILD_MODE: &str = if cfg!(debug_assertions) {
    "debug"
} else {
    "release"
};

pub const VERSION: &str = const_str::concat!("v", PKG_VERSION, "-", BUILD_MODE, "+", GIT_HASH);
