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
