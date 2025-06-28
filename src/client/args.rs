use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::bail;
use clap::{Parser, Subcommand, ValueHint};
use tracing::Level;

use crate::client::types::terrain::AutoApply;
use crate::client::validation::{IdentifierType, validate_identifiers};
use crate::common::constants::{NONE, SHELL, UNSUPPORTED, ZSH, ZSHRC_PATH};

const DEFAULT_SELECTED: &str = "__default__";

/// get default rc path for supported shells
/// if unsupported shell found send UNSUPPORTED. UNSUPPORTED
/// value will be handled inside [shell::get_shell method](crate::client::shell::get_shell)
fn get_default_shell_rc() -> &'static str {
    let shell = std::env::var(SHELL).ok();

    if shell.is_some_and(|s| s.contains(ZSH)) {
        return ZSHRC_PATH;
    }

    UNSUPPORTED
}

/// terrainium
///
/// A command-line utility for environment management
#[derive(Parser, Debug)]
#[command(
    version,
    propagate_version(true),
    args_conflicts_with_subcommands = true
)]
pub struct ClientArgs {
    #[clap(flatten)]
    pub options: Options,

    #[command(subcommand)]
    pub command: Option<Verbs>,
}

#[derive(Parser, Debug)]
pub struct Options {
    /// Creates a configuration file for terrain client
    ///
    /// Location: `~/.config/terrainium/terrainium.toml`
    #[arg(long, conflicts_with = "update_rc")]
    pub create_config: bool,

    /// Adds shell integration to specified rc file
    /// If file is not specified `~/.zshrc` is updated
    #[arg(long,
        num_args = 0..=1,
        default_missing_value = get_default_shell_rc(),
        value_hint = ValueHint::FilePath)]
    pub update_rc: Option<PathBuf>,

    /// Set logging level for validation messages
    ///
    /// For `terrain validate` value is overwritten to debug
    ///
    /// [possible values: trace, debug, info, warn, error]
    #[arg(
        short,
        long,
        default_value = "warn",
        global = true,
        display_order = 100
    )]
    pub log_level: Level,
}

#[derive(Subcommand, Debug)]
pub enum Verbs {
    /// Initialize terrain in current directory
    ///
    /// Creates terrain.toml file
    Init {
        /// Creates terrain.toml in central directory.
        ///
        /// If current directory is /home/user/work/project, then
        /// terrain.toml file is created in
        /// ~/.config/terrainium/terrains/_home_user_work_project/.
        ///
        /// This is useful if user does not want to add terrain.toml
        /// to source control
        #[arg(short, long)]
        central: bool,

        /// Creates terrain.toml with example terrain included.
        #[arg(short = 'x', long)]
        example: bool,

        /// Opens terrain.toml in EDITOR after creation
        ///
        /// Launches editor defined in EDITOR environment variable.
        /// If EDITOR environment variable is not set, 'vi' will be used
        /// as editor.
        #[arg(short, long)]
        edit: bool,
    },

    /// Opens terrain.toml for current directory in EDITOR
    ///
    /// Launches editor defined in EDITOR environment variable.
    /// If EDITOR environment variable is not set, 'vi' will be used
    /// as editor.
    Edit {
        /// Opens editor for active terrain rather than current directory
        #[arg(long)]
        active: bool,
    },

    /// Updates terrain.toml for current directory
    Update {
        /// Sets default biome.
        ///
        /// Will fail if specified biome is not defined before.
        #[arg(short, long, conflicts_with_all = ["biome", "new", "env", "alias", "auto_apply"])]
        set_default: Option<String>,

        /// Updates specified biome
        ///
        /// If not specified default biome will be updated
        #[arg(short, long, group = "biomes", default_value = DEFAULT_SELECTED, hide_default_value = true)]
        biome: BiomeArg,

        /// Creates a new biome
        ///
        /// If -e and -a is used with this option, new biome will be created
        /// with specified environment variables and aliases
        #[arg(short, long, group = "biomes")]
        new: Option<String>,

        /// Updates environment variable to specified biome in '--new' or '--biome'
        ///
        /// Format for environment variable will be ENV_VAR="some value"
        /// ENV_VAR should NOT have double or single quotes around it.
        /// If value does not have spaces in then there is no need for double quotes.
        ///
        /// Multiple environment variable can be specified by -e ENV_VAR1=value1 -e ENV_VAR2=value2
        #[arg(short, long, value_name = "ENV_VAR=\"env value\"")]
        env: Vec<Pair>,

        /// Updates alias to specified biome in '--new' or '--biome'
        ///
        /// Format for alias will be alias_name="some value".
        /// Alias_name should NOT have double or single quotes around it.
        /// If value does not have spaces in then there is no need for double quotes.
        ///
        /// Multiple aliases can be specified by -a alias1=value1 -a alias2=value2
        #[arg(short, long, value_name = "ALIAS=\"alias value\"")]
        alias: Vec<Pair>,

        /// Updates auto_apply value
        #[arg(long, value_enum)]
        auto_apply: Option<AutoApply>,

        /// Backs up terrain.toml before update
        ///
        /// Backs up to terrain.toml.bkp file in same directory as terrain.toml
        #[arg(long)]
        backup: bool,

        /// Updates active terrain rather than current directory
        #[arg(long)]
        active: bool,
    },

    /// Generates required shell scripts for terrainium to work
    ///
    /// MUST be executed if terrain.toml is updated by something other
    /// than `terrain edit`, `terrain update` commands.
    Generate {
        /// generates scripts active terrain rather than current directory
        #[arg(long)]
        active: bool,
    },

    /// Validates the terrain in current directory
    Validate {
        /// Validates the active terrain rather than current directory
        #[arg(long)]
        active: bool,
    },

    /// Fetch the values of the environment for current directory
    ///
    /// If no arguments are provided fetches all the values.
    Get {
        /// Biome to use for environment.
        /// If it is not specified default biome will be used.
        ///
        /// If "none" is used, main terrain will be used without applying any Biome.
        #[arg(short, long, default_value = DEFAULT_SELECTED, hide_default_value = true)]
        biome: BiomeArg,

        /// Fetches all the environment variables
        ///
        /// Cannot be used with `-e`
        #[arg(long, conflicts_with_all = ["env", "json"])]
        envs: bool,

        /// Fetches all the aliases
        ///
        /// Cannot be used with `-a`.
        #[arg(long, conflicts_with_all = ["alias", "json"])]
        aliases: bool,

        /// Fetches specified list of environment variables
        ///
        /// Multiple values can be fetched.
        /// Single instance of `-e` can be supplied with single environment variable to fetch.
        /// .i.e. if multiple values are needed use `-e ENV_VAR1 -e ENV_VAR2`
        /// If value does not exist "!!!DOES_NOT_EXIST!!!" is returned.
        #[arg(short, conflicts_with = "json")]
        env: Vec<String>,

        /// Fetches specified list of aliases
        ///
        /// Multiple values can be fetched.
        /// Single instance of `-a` can be supplied with single alias to fetch.
        /// .i.e. if multiple values are needed use `-a alias1 -a alias2`
        /// If value does not exist "!!!DOES_NOT_EXIST!!!" is returned.
        #[arg(short, conflicts_with = "json")]
        alias: Vec<String>,

        /// Fetches all the constructors
        #[arg(short, long, conflicts_with = "json")]
        constructors: bool,

        /// Fetches all the destructors
        #[arg(short, long, conflicts_with = "json")]
        destructors: bool,

        /// Fetches the current auto_apply value
        #[arg(long, conflicts_with = "json")]
        auto_apply: bool,

        /// Fetches all the values in json format
        #[arg(short, long)]
        json: bool,

        /// Fetches the values for currently active terrain
        #[arg(long)]
        active: bool,

        /// Prints the terrain validation logs
        #[arg(long)]
        debug: bool,
    },

    Enter {
        #[arg(short, long, default_value = DEFAULT_SELECTED)]
        biome: BiomeArg,

        #[arg(long, hide = true)]
        auto_apply: bool,
    },

    Construct {
        #[arg(short, long, default_value = DEFAULT_SELECTED)]
        biome: BiomeArg,
    },

    Destruct {
        #[arg(short, long, default_value = DEFAULT_SELECTED)]
        biome: BiomeArg,
    },

    Exit,

    Status {
        #[arg(short, long)]
        json: bool,
        #[arg(short, long, group = "session")]
        recent: Option<u32>,
        #[arg(short, long, group = "session")]
        session_id: Option<String>,
        #[arg(short, long)]
        terrain_name: Option<String>,
    },

    #[cfg(feature = "terrain-schema")]
    Schema,
}

#[derive(Debug, Clone)]
pub enum BiomeArg {
    None,
    Default,
    Some(String),
}

impl FromStr for BiomeArg {
    type Err = anyhow::Error;

    fn from_str(arg: &str) -> anyhow::Result<Self, Self::Err> {
        match arg.to_lowercase().as_str() {
            NONE => Ok(BiomeArg::None),
            DEFAULT_SELECTED => Ok(BiomeArg::Default),
            _ => Ok(BiomeArg::Some(arg.to_string())),
        }
    }
}

impl From<BiomeArg> for String {
    fn from(val: BiomeArg) -> Self {
        match val {
            BiomeArg::None => NONE.to_string(),
            BiomeArg::Default => DEFAULT_SELECTED.to_string(),
            BiomeArg::Some(selected) => selected,
        }
    }
}

pub struct GetArgs {
    pub json: bool,
    pub biome: BiomeArg,
    pub aliases: bool,
    pub envs: bool,
    pub alias: Vec<String>,
    pub env: Vec<String>,
    pub constructors: bool,
    pub destructors: bool,
    pub auto_apply: bool,
}

impl GetArgs {
    pub(crate) fn empty(&self) -> bool {
        !self.aliases
            && !self.envs
            && self.alias.is_empty()
            && self.env.is_empty()
            && !self.constructors
            && !self.destructors
            && !self.auto_apply
    }
}

pub struct UpdateArgs {
    pub set_default: Option<String>,
    pub biome: BiomeArg,
    pub alias: Vec<Pair>,
    pub env: Vec<Pair>,
    pub new: Option<String>,
    pub backup: bool,
    pub auto_apply: Option<AutoApply>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pair {
    pub key: String,
    pub value: String,
}

impl FromStr for Pair {
    type Err = anyhow::Error;

    fn from_str(pair: &str) -> Result<Self, Self::Err> {
        let pair: Vec<&str> = pair.split("=").collect();

        if pair.len() != 2 {
            bail!("pair of key values should be passed in format <KEY>=<VALUE>.");
        }

        let key = pair.first().unwrap().to_string();
        let value = pair
            .last()
            .unwrap()
            .trim_matches(|c| c == '\'' || c == '"')
            .to_string();

        let mut identifier = BTreeMap::new();
        identifier.insert(key.clone(), value.clone());

        let validation_results =
            validate_identifiers(IdentifierType::Identifier, &identifier, NONE);
        if !validation_results.results_ref().is_empty() {
            validation_results.print_validation_message();
            bail!("env or alias is not valid, please make sure that it is valid.");
        }

        Ok(Pair { key, value })
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use clap::ValueEnum;
    use pretty_assertions::assert_eq;

    use crate::client::args::Pair;
    use crate::client::types::terrain::AutoApply;
    use crate::common::constants::NONE;

    #[test]
    fn pair_from_str() {
        let pair = Pair::from_str("KEY=VALUE").expect("no error to be thrown");
        assert_eq!(
            Pair {
                key: "KEY".to_string(),
                value: "VALUE".to_string()
            },
            pair
        );
    }

    #[test]
    fn pair_from_value_with_double_quotes_and_space() {
        let pair = Pair::from_str("KEY=\"SOME VALUE\"").expect("no error to be thrown");
        assert_eq!(
            Pair {
                key: "KEY".to_string(),
                value: "SOME VALUE".to_string()
            },
            pair
        );
    }

    #[test]
    fn pair_from_value_with_single_quotes_and_space() {
        let pair = Pair::from_str("KEY='SOME VALUE'").expect("no error to be thrown");
        assert_eq!(
            Pair {
                key: "KEY".to_string(),
                value: "SOME VALUE".to_string()
            },
            pair
        );
    }

    #[test]
    fn pair_from_str_throws_error() {
        let err = Pair::from_str("KEY")
            .expect_err("error to be thrown")
            .to_string();
        assert_eq!(
            "pair of key values should be passed in format <KEY>=<VALUE>.",
            err
        );
    }

    #[test]
    fn pair_from_str_throws_error_more_than_one_equals() {
        let err = Pair::from_str("KEY=VALUE=SOMETHING_ELSE")
            .expect_err("error to be thrown")
            .to_string();
        assert_eq!(
            "pair of key values should be passed in format <KEY>=<VALUE>.",
            err
        );
    }

    #[test]
    fn pair_from_str_throws_error_when_validation_fails() {
        let err = Pair::from_str("1KEY=VALUE")
            .expect_err("error to be thrown")
            .to_string();
        assert_eq!(
            "env or alias is not valid, please make sure that it is valid.", err,
            "failed to validate validation error key starting with number"
        );
        let err = Pair::from_str("K#EY=VALUE")
            .expect_err("error to be thrown")
            .to_string();
        assert_eq!(
            "env or alias is not valid, please make sure that it is valid.", err,
            "failed to validate validation error key with invalid chars"
        );
    }

    #[test]
    fn auto_apply_from_str() {
        assert_eq!(
            AutoApply::from_str("enabled", false).expect("to be parsed"),
            AutoApply::Enabled
        );
        assert_eq!(
            AutoApply::from_str("all", false).expect("to be parsed"),
            AutoApply::All
        );
        assert_eq!(
            AutoApply::from_str("replace", false).expect("to be parsed"),
            AutoApply::Replace
        );
        assert_eq!(
            AutoApply::from_str("background", false).expect("to be parsed"),
            AutoApply::Background
        );
        assert_eq!(
            AutoApply::from_str("off", false).expect("to be parsed"),
            AutoApply::default()
        );

        assert_eq!(
            AutoApply::from_str(NONE, false).err().unwrap().to_string(),
            "invalid variant: none"
        );
    }
}
