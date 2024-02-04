use anyhow::Result;
use clap::Parser;
use terrainium::{
    handlers::{
        args::{
            handle_construct, handle_deconstruct, handle_edit, handle_enter, handle_exit,
            handle_get,
        },
        init::handle_init,
        update::handle_update,
    },
    types::args::{TerrainiumArgs, UpdateOpts, Verbs},
};

fn main() -> Result<()> {
    let opts = TerrainiumArgs::parse();

    return match opts.verbs {
        Verbs::Init {
            central,
            full,
            edit,
        } => handle_init(central, full, edit),
        Verbs::Edit => handle_edit(),
        Verbs::Update {
            set_biome,
            opts:
                UpdateOpts {
                    new,
                    biome,
                    env,
                    alias,
                },
            backup,
        } => handle_update(set_biome, new, biome, env, alias, backup),
        Verbs::Get { biome, all, opts } => handle_get(all, biome, opts),
        Verbs::Enter { biome } => handle_enter(biome),
        Verbs::Exit{ biome } => {
            handle_exit(biome)
        },
        Verbs::Construct { biome } => handle_construct(biome),
        Verbs::Deconstruct { biome } => handle_deconstruct(biome),
    };
}
