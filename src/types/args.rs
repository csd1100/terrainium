use std::str::FromStr;

use anyhow::{anyhow, Ok};
use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct TerrainiumArgs {
    #[command(subcommand)]
    pub verbs: Verbs,
}

#[derive(Subcommand, Debug)]
pub enum Verbs {
    Init {
        #[arg(short, long)]
        central: bool,

        #[arg(short, long)]
        full: bool,

        #[arg(short, long)]
        edit: bool,
    },
    Edit,
    Update {
        #[arg(short, long = "set-biome")]
        set_biome: Option<String>,
        #[command(flatten)]
        opts: UpdateOpts,
    },
    Enter {
        #[arg(short, long, default_value = "default")]
        biome: Option<BiomeArg>,
    },
    Exit,
    Construct {
        #[arg(short, long, default_value = "default")]
        biome: Option<BiomeArg>,
    },
    Deconstruct {
        #[arg(short, long, default_value = "default")]
        biome: Option<BiomeArg>,
    },
}

#[derive(Args, Debug)]
#[group(conflicts_with("set_biome"))]
pub struct UpdateOpts {
    #[arg(short, long, default_value = "default")]
    pub biome: Option<BiomeArg>,

    #[arg(short, long)]
    pub env: Option<Vec<Pair>>,

    #[arg(short, long)]
    pub aliases: Option<Vec<Pair>>,
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
            return Err(anyhow!("expected `key=value`pair"));
        }

        let mut drain = values.drain(0..=1);

        let pair: Self = Self {
            key: drain.next().expect("expect to be present").to_string(),
            value: drain.next().expect("expect to be present").to_string(),
        };

        return Ok(pair);
    }
}

#[cfg(test)]
mod test {
    use crate::types::args::TerrainiumArgs;

    #[test]
    fn verify_args() {
        use clap::CommandFactory;
        TerrainiumArgs::command().debug_assert()
    }
}