use anyhow::anyhow;
use clap::{Parser, Subcommand};
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command()]
pub struct ClientArgs {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
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

    Get {
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

        #[arg(short = 'k', long)]
        backup: bool,
    },

    Construct {
        #[arg(short, long)]
        biome: Option<BiomeArg>,
    },
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

pub fn option_string_from(option_biome_arg: &Option<BiomeArg>) -> Option<String> {
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
}

impl GetArgs {
    pub fn empty(&self) -> bool {
        !self.aliases
            && !self.envs
            && self.alias.is_empty()
            && self.env.is_empty()
            && !self.constructors
            && !self.destructors
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

        Ok(Pair {
            key: pair.first().expect("key to be present").to_string(),
            value: pair.get(1).expect("value to be present").to_string(),
        })
    }
}

pub struct UpdateArgs {
    pub set_default: Option<String>,
    pub biome: Option<BiomeArg>,
    pub alias: Vec<Pair>,
    pub env: Vec<Pair>,
    pub new: Option<String>,
    pub backup: bool,
}

#[cfg(test)]
mod test {
    use crate::client::args::Pair;
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
}