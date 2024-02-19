use std::str::FromStr;

use anyhow::{anyhow, Ok};
use clap::{Args, Parser, Subcommand};

use crate::helpers::constants::TERRAINIUM_SELECTED_BIOME;

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
        #[arg(short = 'k', long)]
        backup: bool,

        #[arg(short, long = "set-biome")]
        set_biome: Option<String>,

        #[command(flatten)]
        opts: UpdateOpts,
    },
    Generate,
    Get {
        #[arg(short, long)]
        biome: Option<BiomeArg>,

        #[command(flatten)]
        opts: GetOpts,
    },
    Enter {
        #[arg(short, long)]
        biome: Option<BiomeArg>,
    },
    Exit,
    Construct,
    Deconstruct,
    #[cfg(feature = "terrain-schema")]
    Schema,
}

#[derive(Args, Debug)]
#[group(conflicts_with("set_biome"))]
pub struct UpdateOpts {
    #[arg(short, long, group = "for")]
    pub new: Option<String>,

    #[arg(short, long, group = "for")]
    pub biome: Option<BiomeArg>,

    #[arg(short, long)]
    pub env: Option<Vec<Pair>>,

    #[arg(short, long)]
    pub alias: Option<Vec<Pair>>,
}

#[derive(Args, Debug)]
pub struct GetOpts {
    #[arg(long = "alias", group = "aliases")]
    pub alias_all: bool,

    #[arg(short, group = "aliases")]
    pub alias: Option<Vec<String>>,

    #[arg(long = "env", group = "envs")]
    pub env_all: bool,

    #[arg(short, group = "envs")]
    pub env: Option<Vec<String>>,

    #[arg(short, long)]
    pub constructors: bool,

    #[arg(short, long)]
    pub destructors: bool,
}

impl GetOpts {
    pub fn is_empty(&self) -> bool {
        if !self.destructors
            && !self.constructors
            && !self.alias_all
            && !self.env_all
            && self.env.is_none()
            && self.alias.is_none()
        {
            return true;
        }
        false
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BiomeArg {
    Default,
    None,
    Value(String),
    Current(String),
}

impl FromStr for BiomeArg {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(BiomeArg::None),
            "default" => Ok(BiomeArg::Default),
            "current" => {
                if let std::result::Result::Ok(current) = std::env::var(TERRAINIUM_SELECTED_BIOME) {
                    Ok(BiomeArg::Current(current))
                } else {
                    Err(anyhow!("no active biome found"))
                }
            }
            _ => Ok(BiomeArg::Value(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pair {
    pub key: String,
    pub value: String,
}

impl FromStr for Pair {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut values: Vec<_> = s.split('=').collect();
        if values.len() != 2 {
            return Err(anyhow!("expected a `key=value` pair"));
        }

        let mut drain = values.drain(0..=1);

        let pair: Self = Self {
            key: drain.next().expect("expect to be present").to_string(),
            value: drain.next().expect("expect to be present").to_string(),
        };

        Ok(pair)
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use anyhow::Result;

    use crate::types::args::{BiomeArg, Pair, TerrainiumArgs};

    #[test]
    fn verify_args() {
        use clap::CommandFactory;
        TerrainiumArgs::command().debug_assert()
    }

    #[test]
    fn str_to_pair() -> Result<()> {
        let expected = Pair {
            key: "test".to_string(),
            value: "val".to_string(),
        };
        let val = "test=val";

        let actual: Pair = Pair::from_str(val)?;
        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn str_to_pair_returns_error_for_bad_string() -> Result<()> {
        let expected = "expected a `key=value` pair";
        let val = "testval";

        let actual = Pair::from_str(val).unwrap_err().to_string();
        assert_eq!(expected, actual);

        Ok(())
    }

    #[test]
    fn str_to_biomearg() -> Result<()> {
        let expected = BiomeArg::None;
        let val = "none";

        let actual = BiomeArg::from_str(val)?;
        assert_eq!(expected, actual);

        let expected = BiomeArg::Default;
        let val = "default";

        let actual = BiomeArg::from_str(val)?;
        assert_eq!(expected, actual);

        let expected = BiomeArg::Value("test".to_string());
        let val = "test";

        let actual = BiomeArg::from_str(val)?;
        assert_eq!(expected, actual);
        Ok(())
    }
}
