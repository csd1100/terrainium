use anyhow::bail;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub enum HistoryArg {
    Current,
    Recent,
    Recent1,
    Recent2,
}

impl FromStr for HistoryArg {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "recent" => Ok(HistoryArg::Recent),
            "recent~1" => Ok(HistoryArg::Recent1),
            "recent~2" => Ok(HistoryArg::Recent2),
            "current" => Ok(HistoryArg::Current),
            _ => bail!("failed to parse history argument from: {s}"),
        }
    }
}
