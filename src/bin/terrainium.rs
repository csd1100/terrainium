use anyhow::Result;
use clap::Parser;
#[cfg(feature = "terrain-schema")]
use terrainium::handlers::schema;
use terrainium::helpers::utils::{get_cwd, get_home_dir, get_paths};
use terrainium::{
    handlers::{construct, deconstruct, edit, enter, exit, generate, get, init, status, update},
    types::args::{TerrainiumArgs, Verbs},
};

fn main() -> Result<()> {
    let opts = TerrainiumArgs::parse();
    let paths = get_paths(get_home_dir()?, get_cwd()?)?;

    match opts.verbs {
        Verbs::Init {
            central,
            example,
            edit,
        } => init::handle(central, example, edit, &paths),
        Verbs::Edit => edit::handle(&paths),
        Verbs::Update {
            set_default_biome,
            opts,
            backup,
        } => update::handle(set_default_biome, opts, backup, &paths),
        Verbs::Get { biome, opts } => get::handle(biome, opts, &paths),
        Verbs::Generate => generate::handle(&paths),
        Verbs::Enter { biome } => enter::handle(biome, &paths),
        Verbs::Exit => exit::handle(),
        Verbs::Construct => construct::handle(&paths),
        Verbs::Deconstruct => deconstruct::handle(&paths),
        Verbs::Status {
            session,
            list_processes,
            process_id,
        } => status::handle(session, list_processes, process_id),
        #[cfg(feature = "terrain-schema")]
        Verbs::Schema => schema::handle(),
    }
}
