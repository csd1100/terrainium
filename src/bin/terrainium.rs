use anyhow::Result;
use clap::Parser;
use terrainium::{
    parser::args::{
        handle_construct, handle_deconstruct, handle_edit, handle_enter, handle_exit, handle_init,
        handle_update,
    },
    types::args::{Args, Verbs},
};

fn main() -> Result<()> {
    let opts = Args::parse();

    return match opts.verbs {
        Verbs::Init {
            central,
            full,
            edit,
        } => handle_init(central, full, edit),
        Verbs::Edit => handle_edit(),
        Verbs::Update {
            set_biome,
            biome,
            env,
            alias,
            construct,
            destruct,
        } => handle_update(set_biome, biome, env, alias, construct, destruct),
        Verbs::Enter { biome } => handle_enter(biome),
        Verbs::Exit => handle_exit(),
        Verbs::Construct { biome } => handle_construct(biome),
        Verbs::Deconstruct { biome } => handle_deconstruct(biome),
    };
}
