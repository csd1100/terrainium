use anyhow::Result;
use clap::Parser;
#[cfg(feature = "terrain-schema")]
use terrainium::handlers::schema;
use terrainium::{
    handlers::{construct, deconstruct, edit, enter, exit, generate, get, init, status, update},
    types::args::{TerrainiumArgs, Verbs},
};

fn main() -> Result<()> {
    let opts = TerrainiumArgs::parse();

    match opts.verbs {
        Verbs::Init {
            central,
            example,
            edit,
        } => init::handle(central, example, edit),
        Verbs::Edit => edit::handle(),
        Verbs::Update {
            set_biome,
            opts,
            backup,
        } => update::handle(set_biome, opts, backup),
        Verbs::Get { biome, opts } => get::handle(biome, opts),
        Verbs::Generate => generate::handle(),
        Verbs::Enter { biome } => enter::handle(biome),
        Verbs::Exit => exit::handle(),
        Verbs::Construct => construct::handle(),
        Verbs::Deconstruct => deconstruct::handle(),
        Verbs::Status {
            session,
            list_processes,
            process_id,
        } => status::handle(session, list_processes, process_id),
        #[cfg(feature = "terrain-schema")]
        Verbs::Schema => schema::handle(),
    }
}
