use anyhow::Result;
use clap::Parser;
use terrainium::{
    handlers::{
        args::{handle_construct, handle_deconstruct, handle_edit, handle_enter, handle_exit},
        init::handle_init,
        update::handle_update,
    },
    types::args::{TerrainiumArgs, UpdateOpts, Verbs},
};

fn main() -> Result<()> {
    let opts = TerrainiumArgs::parse();
    println!("{:?}", opts);

    return match opts.verbs {
        Verbs::Init {
            central,
            full,
            edit,
        } => handle_init(central, full, edit),
        Verbs::Edit => handle_edit(),
        Verbs::Update {
            set_biome: _,
            opts:
                UpdateOpts {
                    biome: _,
                    env: _,
                    aliases: _,
                },
        } => handle_update(),
        Verbs::Enter { biome } => handle_enter(biome),
        Verbs::Exit => handle_exit(),
        Verbs::Construct { biome } => handle_construct(biome),
        Verbs::Deconstruct { biome } => handle_deconstruct(biome),
    };
}
