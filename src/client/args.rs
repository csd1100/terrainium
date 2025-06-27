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
/// if unsupported send UNSUPPORTED. As, UNSUPPORTED
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
/// a command-line utility for environment management
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
    /// creates a configuration file for terrain client
    ///
    /// location: `~/.config/terrainium/terrainium.toml`
    #[arg(long, conflicts_with = "update_rc")]
    pub create_config: bool,

    /// adds shell integration to specified rc file
    /// if file is not specified `~/.zshrc` is updated
    #[arg(long,
        num_args = 0..=1,
        default_missing_value = get_default_shell_rc(),
        value_hint = ValueHint::FilePath)]
    pub update_rc: Option<PathBuf>,

    /// set logging level for validation messages
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
    /// initialize terrain in current directory
    ///
    /// creates terrain.toml file
    Init {
        /// creates terrain.toml in central directory.
        ///
        /// if current directory is /home/user/work/project, then
        /// terrain.toml file is created in
        /// ~/.config/terrainium/terrains/_home_user_work_project/.
        ///
        /// This is useful if user does not want to add terrain.toml
        /// to source control
        #[arg(short, long)]
        central: bool,

        /// creates terrain.toml with example terrain included.
        #[arg(short = 'x', long)]
        example: bool,

        /// opens terrain.toml in EDITOR after creation
        ///
        /// launches editor defined in EDITOR environment variable.
        /// if EDITOR environment variable is not set, 'vi' will be used
        /// as editor.
        #[arg(short, long)]
        edit: bool,
    },

    /// opens terrain.toml for current directory in EDITOR
    ///
    /// launches editor defined in EDITOR environment variable.
    /// if EDITOR environment variable is not set, 'vi' will be used
    /// as editor.
    Edit {
        /// opens editor for active terrain rather than current directory
        #[arg(long)]
        active: bool,
    },

    /// updates terrain.toml for current directory
    Update {
        /// sets default biome.
        ///
        /// will fail if specified biome is not defined before.
        #[arg(short, long, conflicts_with_all = ["biome", "new", "env", "alias", "auto_apply"])]
        set_default: Option<String>,

        /// updates specified biome
        ///
        /// if not specified default biome will be updated
        #[arg(short, long, group = "biomes", default_value = DEFAULT_SELECTED, hide_default_value = true)]
        biome: BiomeArg,

        /// creates a new biome
        ///
        /// if -e and -a is used with this option, new biome will be created
        /// with specified environment variables and aliases
        #[arg(short, long, group = "biomes")]
        new: Option<String>,

        /// updates environment variable to specified biome in '--new' or '--biome'
        ///
        /// format for environment variable will be ENV_VAR="some value"
        ///
        /// multiple environment variable can be specified by -e ENV_VAR1=value1 -e ENV_VAR2=value2
        #[arg(short, long, value_name = "ENV_VAR=\"env value\"")]
        env: Vec<Pair>,

        /// updates alias to specified biome in '--new' or '--biome'
        ///
        /// format for alias will be alias="some value".
        ///
        /// multiple aliases can be specified by -a alias1=value1 -a alias2=value2
        #[arg(short, long, value_name = "ALIAS=\"alias value\"")]
        alias: Vec<Pair>,

        /// updates auto_apply value
        #[arg(long, value_enum)]
        auto_apply: Option<AutoApply>,

        /// backs up terrain.toml before update
        ///
        /// backs up to terrain.toml.bkp file in same directory as terrain.toml
        #[arg(long)]
        backup: bool,

        /// updates active terrain rather than current directory
        #[arg(long)]
        active: bool,
    },

    Generate {
        #[arg(long)]
        active: bool,
    },

    Validate,

    Get {
        #[arg(long)]
        active: bool,

        #[arg(long)]
        debug: bool,

        #[arg(short, long, name = "json")]
        json: bool,

        #[arg(short, long, default_value = DEFAULT_SELECTED, conflicts_with = "json")]
        biome: BiomeArg,

        #[arg(long, group = "get_alias", conflicts_with = "json")]
        aliases: bool,

        #[arg(long, group = "get_env", conflicts_with = "json")]
        envs: bool,

        #[arg(short, group = "get_alias", conflicts_with = "json")]
        alias: Vec<String>,

        #[arg(short, group = "get_env", conflicts_with = "json")]
        env: Vec<String>,

        #[arg(short, long, conflicts_with = "json")]
        constructors: bool,

        #[arg(short, long, conflicts_with = "json")]
        destructors: bool,

        #[arg(long, conflicts_with = "json")]
        auto_apply: bool,
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
