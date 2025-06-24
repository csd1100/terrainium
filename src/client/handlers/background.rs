use crate::client::args::BiomeArg;
#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::environment::Environment;
use crate::client::types::proto::ProtoRequest;
use crate::client::types::terrain::Terrain;
use crate::common::constants::{CONSTRUCTORS, DESTRUCTORS};
use crate::common::types::paths::get_terrainiumd_paths;
use crate::common::types::pb;
use anyhow::{Context as AnyhowContext, Result};

pub async fn handle(
    context: Context,
    biome: BiomeArg,
    terrain: Terrain,
    is_constructor: bool,
    timestamp: String,
    client: Option<Client>,
) -> Result<()> {
    let environment = Environment::from(&terrain, biome, context.terrain_dir())
        .context("failed to generate environment")?;

    let mut client: Client = if let Some(client) = client {
        client
    } else {
        Client::new(get_terrainiumd_paths().socket()).await?
    };

    let name = environment.name().to_owned();
    let biome = environment.selected_biome().to_owned();
    match execute_request(&context, environment, is_constructor, timestamp)? {
        None => {
            println!(
                "no background {} were found for {}({})",
                if is_constructor {
                    CONSTRUCTORS
                } else {
                    DESTRUCTORS
                },
                name,
                biome
            );
        }
        Some(request) => {
            client.request(ProtoRequest::Execute(request)).await?;
        }
    }

    Ok(())
}

pub(crate) fn execute_request(
    context: &Context,
    environment: Environment,
    is_constructor: bool,
    timestamp: String,
) -> Result<Option<pb::Execute>> {
    let commands = if is_constructor {
        environment.constructors()
    } else {
        environment.destructors()
    };

    let commands: Vec<pb::Command> = commands
        .to_proto_commands()
        .context("failed to convert commands")?;

    if commands.is_empty() {
        return Ok(None);
    }

    Ok(Some(pb::Execute {
        session_id: context.session_id(),
        terrain_name: environment.name().to_string(),
        biome_name: environment.selected_biome().to_string(),
        terrain_dir: context.terrain_dir().to_string_lossy().to_string(),
        toml_path: context.toml_path().to_string_lossy().to_string(),
        is_constructor,
        timestamp,
        envs: environment.envs(),
        commands,
    }))
}
