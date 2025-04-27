use crate::client::types::terrain::AutoApply;
use crate::client::validation::{validate_identifiers, IdentifierType};
use anyhow::anyhow;
use clap::{Parser, Subcommand};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;
use tracing::Level;

#[derive(Parser, Debug)]
#[command(args_conflicts_with_subcommands = true)]
pub struct ClientArgs {
    #[clap(flatten)]
    pub options: Options,

    #[command(subcommand)]
    pub command: Option<Verbs>,
}

#[derive(Parser, Debug)]
pub struct Options {
    #[arg(long, group = "update-rc")]
    pub update_rc: bool,

    #[arg(long, group = "update-rc")]
    pub update_rc_path: Option<PathBuf>,

    #[arg(short, long, default_value = "warn", global = true)]
    pub log_level: Level,

    #[arg(long)]
    pub create_config: bool,
}

#[derive(Subcommand, Debug)]
pub enum Verbs {
    Init {
        #[arg(short, long)]
        central: bool,

        #[arg(short = 'x', long)]
        example: bool,

        #[arg(short, long)]
        edit: bool,
    },

    Edit,

    Generate,

    Validate,

    Get {
        #[arg(long)]
        debug: bool,

        #[arg(short, long)]
        biome: Option<BiomeArg>,

        #[arg(long, group = "get_alias")]
        aliases: bool,

        #[arg(long, group = "get_env")]
        envs: bool,

        #[arg(short, group = "get_alias")]
        alias: Vec<String>,

        #[arg(short, group = "get_env")]
        env: Vec<String>,

        #[arg(short, long)]
        constructors: bool,

        #[arg(short, long)]
        destructors: bool,

        #[arg(long)]
        auto_apply: bool,
    },

    Update {
        #[arg(short, long, groups = ["update_biome" , "update"])]
        set_default: Option<String>,

        #[arg(short, long, group = "update_biome")]
        biome: Option<BiomeArg>,

        #[arg(short, long, group = "update")]
        alias: Vec<Pair>,

        #[arg(short, long, group = "update")]
        env: Vec<Pair>,

        #[arg(short, long, group = "update_biome")]
        new: Option<String>,

        #[arg(long)]
        auto_apply: Option<AutoApply>,

        #[arg(short = 'k', long)]
        backup: bool,
    },

    Construct {
        #[arg(short, long)]
        biome: Option<BiomeArg>,
    },

    Destruct {
        #[arg(short, long)]
        biome: Option<BiomeArg>,
    },

    Enter {
        #[arg(short, long)]
        biome: Option<BiomeArg>,

        #[arg(long, hide = true)]
        auto_apply: bool,
    },

    Exit,

    #[cfg(feature = "terrain-schema")]
    Schema,
}

#[derive(Debug, Clone)]
pub enum BiomeArg {
    None,
    Current,
    Some(String),
}

impl FromStr for BiomeArg {
    type Err = anyhow::Error;

    fn from_str(arg: &str) -> anyhow::Result<Self, Self::Err> {
        match arg {
            "none" => Ok(BiomeArg::None),
            "current" => Ok(BiomeArg::Current),
            _ => Ok(BiomeArg::Some(arg.to_string())),
        }
    }
}

impl From<BiomeArg> for String {
    fn from(val: BiomeArg) -> Self {
        match val {
            BiomeArg::None => "none".to_string(),
            BiomeArg::Current => "current".to_string(),
            BiomeArg::Some(selected) => selected,
        }
    }
}

pub(crate) fn option_string_from(option_biome_arg: &Option<BiomeArg>) -> Option<String> {
    option_biome_arg.clone().map(|selected| selected.into())
}

pub struct GetArgs {
    pub biome: Option<BiomeArg>,
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
            return Err(anyhow!(
                "pair of key values should be passed in format <KEY>=<VALUE>."
            ));
        }

        let mut env = BTreeMap::new();
        env.insert(pair[0].to_string(), pair[1].to_string());

        let validation_results = validate_identifiers(IdentifierType::Identifier, &env, "none");
        if !validation_results.results_ref().is_empty() {
            validation_results.print_validation_message();
            return Err(anyhow!(
                "env or alias is not valid, please make sure that it is valid."
            ));
        }

        Ok(Pair {
            key: pair.first().expect("key to be present").to_string(),
            value: pair.get(1).expect("value to be present").to_string(),
        })
    }
}

impl FromStr for AutoApply {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "enable" => Ok(AutoApply::enabled()),
            "replace" => Ok(AutoApply::replace()),
            "background" => Ok(AutoApply::background()),
            "all" => Ok(AutoApply::all()),
            "off" => Ok(AutoApply::default()),
            _ => Err(anyhow!("failed to parse auto_apply argument from: {s}")),
        }
    }
}

pub struct UpdateArgs {
    pub set_default: Option<String>,
    pub biome: Option<BiomeArg>,
    pub alias: Vec<Pair>,
    pub env: Vec<Pair>,
    pub new: Option<String>,
    pub backup: bool,
    pub auto_apply: Option<AutoApply>,
}

#[cfg(test)]
mod tests {
    use crate::client::args::Pair;
    use crate::client::types::terrain::AutoApply;
    use std::str::FromStr;

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
            AutoApply::from_str("enable").expect("to be parsed"),
            AutoApply::enabled()
        );
        assert_eq!(
            AutoApply::from_str("all").expect("to be parsed"),
            AutoApply::all()
        );
        assert_eq!(
            AutoApply::from_str("replace").expect("to be parsed"),
            AutoApply::replace()
        );
        assert_eq!(
            AutoApply::from_str("background").expect("to be parsed"),
            AutoApply::background()
        );
        assert_eq!(
            AutoApply::from_str("off").expect("to be parsed"),
            AutoApply::default()
        );

        assert_eq!(
            AutoApply::from_str("none").err().unwrap().to_string(),
            "failed to parse auto_apply argument from: none"
        );
    }
}
