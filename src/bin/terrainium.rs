use anyhow::Result;
use clap::Parser;
use terrainium::types::args::Args;

fn main() -> Result<()> {
    let opts = Args::parse();
    println!("{:?}", opts);
    return Ok(());
}
