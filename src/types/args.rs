use std::str::FromStr;

use anyhow::{anyhow, Ok};
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    #[command(subcommand)]
    pub verbs: Verbs,
}

#[derive(Subcommand, Debug)]
pub enum Verbs {
    Init {
        #[arg(short = 'c', long = "central")]
        central: bool,
        full: bool,
        edit: bool,
    },
    Edit,
    Update {
        #[arg(short, long)]
        biome: Option<BiomeArg>,
        #[arg(short, long)]
        env: Option<Vec<Pair>>,
        #[arg(short, long)]
        alias: Option<Vec<Pair>>,
        #[arg(short, long, requires = "deconstruct")]
        construct: Option<Pair>,
        #[arg(short, long, requires = "construct")]
        deconstruct: Option<Pair>,
    },
    Enter {
        #[arg(short, long)]
        biome: Option<BiomeArg>,
    },
    Exit,
    Construct {
        #[arg(short, long)]
        biome: Option<BiomeArg>,
    },
    Deconstruct {
        #[arg(short, long)]
        biome: Option<BiomeArg>,
    },
}

#[derive(Debug, Clone)]
pub enum BiomeArg {
    Default,
    None,
    Value(String),
}

impl FromStr for BiomeArg {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => {
                return Ok(BiomeArg::None);
            }
            "default" => {
                return Ok(BiomeArg::Default);
            }
            _ => {
                return Ok(BiomeArg::Value(s.to_string()));
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Pair {
    pub key: String,
    pub value: String,
}

impl FromStr for Pair {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut values: Vec<_> = s.split("=").collect();
        if values.len() != 2 {
            return Err(anyhow!(
                "Invalid value passed for argument, expected `key=value`pair"
            ));
        }

        let mut drain = values.drain(0..=1);

        let pair: Self = Self {
            key: drain.next().expect("expect to be present").to_string(),
            value: drain.next().expect("expect to be present").to_string(),
        };

        return Ok(pair);
    }
}
