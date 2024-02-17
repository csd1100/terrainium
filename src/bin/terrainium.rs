use anyhow::Result;
use clap::Parser;
use terrainium::{
    handlers::{construct, deconstruct, edit, enter, exit, generate, get, init, update},
    types::args::{TerrainiumArgs, Verbs},
};

fn main() -> Result<()> {
    let opts = TerrainiumArgs::parse();

    match opts.verbs {
        Verbs::Init {
            central,
            full,
            edit,
        } => init::handle(central, full, edit),
        Verbs::Edit => edit::handle(),
        Verbs::Update {
            set_biome,
            opts,
            backup,
        } => update::handle(set_biome, opts, backup),
        Verbs::Get { biome, all, opts } => get::handle(all, biome, opts),
        Verbs::Enter { biome } => enter::handle(biome),
        Verbs::Exit => exit::handle(),
        Verbs::Construct { biome } => construct::handle(biome),
        Verbs::Deconstruct { biome } => deconstruct::handle(biome),
        Verbs::Generate => generate::handle(),
    }
}
