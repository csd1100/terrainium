use crate::client::args::BiomeArg;
use crate::client::shell::Shell;
#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::environment::Environment;
use crate::client::types::proto::ProtoRequest;
use crate::client::types::terrain::Terrain;
use crate::common::constants::{
    DEBUG_PATH, PATH, TERRAINIUMD_SOCKET, TERRAINIUM_DEV, TERRAIN_AUTO_APPLY, TERRAIN_ENABLED,
    TERRAIN_SESSION_ID, TRUE,
};
use crate::common::types::pb;
use crate::common::utils::timestamp;
use anyhow::{bail, Context as AnyhowContext, Result};
use std::path::PathBuf;
use uuid::Uuid;

pub async fn handle(
    context: Context,
    biome: BiomeArg,
    terrain: Terrain,
    auto_apply: bool,
    client: Option<Client>,
) -> Result<()> {
    let context = context.set_session_id(Uuid::new_v4().to_string());

    let mut environment = Environment::from(&terrain, biome, context.terrain_dir())
        .context("failed to generate environment")?;

    let zsh_envs = context
        .shell()
        .generate_envs(&context, environment.selected_biome())?;
    environment.append_envs(zsh_envs);

    environment.insert_env(TERRAIN_ENABLED.to_string(), TRUE.to_string());
    environment.insert_env(
        TERRAIN_SESSION_ID.to_string(),
        context
            .session_id()
            .expect("session id to be set")
            .to_string(),
    );
    if auto_apply {
        environment.insert_env(
            TERRAIN_AUTO_APPLY.to_string(),
            environment.auto_apply().into(),
        );
    }

    let is_background = !auto_apply || environment.auto_apply().is_background();

    let mut shell_envs = environment.envs();
    if cfg!(debug_assertions) && shell_envs.get(TERRAINIUM_DEV).is_some_and(|v| v == "true") {
        let path = std::env::var(PATH).context("expected environment variable PATH")?;
        let debug_path = context.terrain_dir().join(DEBUG_PATH);
        let path = format!("{}:{path}", debug_path.display());
        shell_envs.insert(PATH.to_string(), path);
    }

    let result = tokio::join!(
        context.shell().spawn(shell_envs),
        send_activate_request(client, &context, environment, is_background)
    );

    if let Err(e) = result.0 {
        bail!("failed to spawn shell while entering terrain environment: {e}");
    } else {
        let exit = result.0?;
        if !exit.success() {
            bail!("spawned shell exited with code: {:?}", exit.code());
        }
    }
    if let Err(e) = result.1 {
        bail!("failed to spawn background processes while entering terrain environment: {e}");
    }

    Ok(())
}

async fn send_activate_request(
    client: Option<Client>,
    context: &Context,
    environment: Environment,
    is_background: bool,
) -> Result<()> {
    let mut client = if let Some(client) = client {
        client
    } else {
        Client::new(PathBuf::from(TERRAINIUMD_SOCKET)).await?
    };

    client
        .request(ProtoRequest::Activate(activate_request(
            context,
            environment,
            is_background,
        )?))
        .await?;

    Ok(())
}

fn activate_request(
    context: &Context,
    environment: Environment,
    is_background: bool,
) -> Result<pb::Activate> {
    let timestamp = timestamp();

    let constructors = if is_background {
        let commands: Vec<pb::Command> = environment
            .constructors()
            .to_proto_commands(environment.envs())
            .context("failed to convert commands")?;

        Some(pb::Execute {
            session_id: Some(
                context
                    .session_id()
                    .expect("session id to be set")
                    .to_string(),
            ),
            terrain_name: environment.name().to_string(),
            biome_name: environment.selected_biome().to_string(),
            terrain_dir: context.terrain_dir().to_string_lossy().to_string(),
            is_constructor: true,
            toml_path: context.toml_path().display().to_string(),
            timestamp: timestamp.clone(),
            commands,
        })
    } else {
        None
    };

    Ok(pb::Activate {
        session_id: context
            .session_id()
            .expect("session id to be set")
            .to_string(),
        terrain_name: environment.name().to_string(),
        biome_name: environment.selected_biome().to_string(),
        terrain_dir: context.terrain_dir().to_string_lossy().to_string(),
        toml_path: context.toml_path().display().to_string(),
        start_timestamp: timestamp,
        is_background,
        constructors,
    })
}

#[cfg(test)]
mod tests {}
