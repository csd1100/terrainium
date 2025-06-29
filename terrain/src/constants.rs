use const_str::concat;

pub const SHELL: &str = "SHELL";
pub const HOME: &str = "~/";
pub const FPATH: &str = "FPATH";

pub const ZSH: &str = "zsh";
pub const ZSHRC: &str = ".zshrc";
pub const ZSHRC_PATH: &str = concat!(HOME, ZSHRC);

pub const TERRAIN_NAME: &str = "TERRAIN_NAME";
pub const TERRAIN_DIR: &str = "TERRAIN_DIR";
pub const TERRAIN_SESSION_ID: &str = "TERRAIN_SESSION_ID";
pub const TERRAIN_AUTO_APPLY: &str = "TERRAIN_AUTO_APPLY";
pub const TERRAIN_SELECTED_BIOME: &str = "TERRAIN_SELECTED_BIOME";

pub const TERRAIN_INIT_FN: &str = "terrain_init";
pub const TERRAIN_INIT_SCRIPT: &str = "TERRAIN_INIT_SCRIPT";

pub const NONE: &str = "none";
pub const EXAMPLE_BIOME: &str = "example_biome";
pub const TERRAINIUM: &str = "terrainium";

pub const EDITOR: &str = "EDITOR";
pub const ENV_VAR: &str = "ENV_VAR";
pub const NESTED_POINTER: &str = "NESTED_POINTER";
pub const NULL_POINTER: &str = "NULL_POINTER";
pub const PAGER: &str = "PAGER";
pub const POINTER_ENV_VAR: &str = "POINTER_ENV_VAR";

pub const TENTER: &str = "tenter";
pub const TEXIT: &str = "texit";

pub const UNSUPPORTED: &str = "UNSUPPORTED";
