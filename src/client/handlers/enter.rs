use crate::client::args::BiomeArg;
use crate::client::handlers::background::execute_request;
use crate::client::shell::Shell;
#[mockall_double::double]
use crate::client::types::client::Client;
use crate::client::types::context::Context;
use crate::client::types::environment::Environment;
use crate::client::types::proto::ProtoRequest;
use crate::client::types::terrain::Terrain;
use crate::common::constants::{
    DEBUG_PATH, PATH, TERRAINIUM_DEV, TERRAIN_AUTO_APPLY, TERRAIN_ENABLED, TERRAIN_SESSION_ID, TRUE,
};
use crate::common::types::paths::get_terrainiumd_paths;
use crate::common::types::pb;
use crate::common::utils::timestamp;
use anyhow::{bail, Context as AnyhowContext, Result};
use std::sync::Arc;
use uuid::Uuid;

pub async fn handle(
    context: Context,
    biome: BiomeArg,
    terrain: Terrain,
    is_auto_apply: bool,
    client: Option<Client>,
) -> Result<()> {
    let context = if cfg!(test) {
        // tests should have already set session id
        if context.session_id().is_none() {
            bail!("session_id is expected when running tests");
        }
        context
    } else {
        // uuid is randomly generated
        context.set_session_id(&Uuid::new_v4().to_string())
    };

    let mut environment = Environment::from(&terrain, biome, context.terrain_dir())
        .context("failed to generate environment")?;

    let zsh_envs = context
        .shell()
        .generate_envs(&context, environment.selected_biome())?;
    environment.append_envs(zsh_envs);

    environment.insert_env(TERRAIN_ENABLED.to_string(), TRUE.to_string());
    environment.insert_env(
        TERRAIN_SESSION_ID.to_string(),
        context.session_id().expect("session id to be set"),
    );
    if is_auto_apply {
        environment.insert_env(
            TERRAIN_AUTO_APPLY.to_string(),
            environment.auto_apply().to_string(),
        );
    }

    let is_background = !is_auto_apply
        || (context.config().auto_apply() && environment.auto_apply().is_background_enabled());

    let mut shell_envs = environment.envs();
    if cfg!(debug_assertions) && shell_envs.get(TERRAINIUM_DEV).is_some_and(|v| v == "true") {
        let path = std::env::var(PATH).context("expected environment variable PATH")?;
        let debug_path = context.terrain_dir().join(DEBUG_PATH);
        let path = format!("{}:{path}", debug_path.display());
        shell_envs.insert(PATH.to_string(), path);
    }

    let result = tokio::join!(
        context.shell().spawn(Some(Arc::new(shell_envs))),
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
        Client::new(get_terrainiumd_paths().socket()).await?
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

    let terrain_name = environment.name().to_owned();
    let biome_name = environment.selected_biome().to_owned();

    let constructors = if is_background {
        execute_request(context, environment, true, timestamp.clone())
            .context("failed to create constructors request")?
    } else {
        None
    };

    Ok(pb::Activate {
        session_id: context.session_id().expect("session id to be set"),
        terrain_name,
        biome_name,
        terrain_dir: context.terrain_dir().to_string_lossy().to_string(),
        toml_path: context.toml_path().display().to_string(),
        start_timestamp: timestamp,
        is_background,
        constructors,
    })
}

#[cfg(test)]
mod tests {
    use crate::client::args::BiomeArg;
    use crate::client::test_utils::assertions::client::ExpectClient;
    use crate::client::test_utils::assertions::zsh::ExpectZSH;
    use crate::client::test_utils::expected_env_vars_none;
    use crate::client::types::context::Context;
    use crate::client::types::proto::ProtoRequest;
    use crate::client::types::terrain::{AutoApply, Terrain};
    use crate::common::constants::{
        FPATH, NONE, TERRAIN_AUTO_APPLY, TERRAIN_DIR, TERRAIN_ENABLED, TERRAIN_INIT_FN,
        TERRAIN_INIT_SCRIPT, TERRAIN_SESSION_ID, TERRAIN_TOML, TEST_TIMESTAMP, TRUE,
    };
    use crate::common::execute::MockExecutor;
    use crate::common::test_utils::{
        expected_activate_request_example_biome, expected_envs_with_activate_example_biome,
        TEST_FPATH, TEST_TERRAIN_NAME,
    };
    use crate::common::test_utils::{TEST_CENTRAL_DIR, TEST_SESSION_ID, TEST_TERRAIN_DIR};
    use crate::common::types::pb;
    use std::collections::BTreeMap;
    use std::path::Path;

    fn expected_envs_with_activate_none(
        is_auto_apply: bool,
        auto_apply: &AutoApply,
    ) -> BTreeMap<String, String> {
        let script = format!("{TEST_CENTRAL_DIR}/scripts/terrain-none.zwc");

        let mut envs = expected_env_vars_none(Path::new(TEST_TERRAIN_DIR));
        envs.insert(FPATH.to_string(), format!("{}:{}", script, TEST_FPATH));
        envs.insert(TERRAIN_INIT_FN.to_string(), "terrain-none.zsh".to_string());
        envs.insert(TERRAIN_INIT_SCRIPT.to_string(), script);
        envs.insert(TERRAIN_DIR.to_string(), TEST_TERRAIN_DIR.to_string());
        envs.insert(TERRAIN_ENABLED.to_string(), TRUE.to_string());
        envs.insert(TERRAIN_SESSION_ID.to_string(), TEST_SESSION_ID.to_string());
        if is_auto_apply {
            envs.insert(TERRAIN_AUTO_APPLY.to_string(), auto_apply.to_string());
        }
        envs
    }

    fn expected_activate_request_none(is_background: bool) -> pb::Activate {
        pb::Activate {
            session_id: TEST_SESSION_ID.to_string(),
            terrain_name: TEST_TERRAIN_NAME.to_string(),
            biome_name: NONE.to_string(),
            terrain_dir: TEST_TERRAIN_DIR.to_string(),
            toml_path: format!("{TEST_TERRAIN_DIR}/{TERRAIN_TOML}"),
            start_timestamp: TEST_TIMESTAMP.to_string(),
            is_background,
            constructors: None,
        }
    }

    #[tokio::test]
    async fn spawns_shell_and_sends_activate_request_auto_apply_all() {
        let is_background = true;
        let auto_apply = AutoApply::All;
        let is_auto_apply = true;

        let terrain_dir = Path::new(TEST_TERRAIN_DIR);
        let central_dir = Path::new(TEST_CENTRAL_DIR);

        let executor = ExpectZSH::with(MockExecutor::default(), terrain_dir)
            .get_fpath()
            .spawn_shell(
                expected_envs_with_activate_example_biome(is_auto_apply, &auto_apply),
                0,
                false,
                "".to_string(),
            )
            .successfully();

        let context = Context::build(terrain_dir, central_dir, false, executor)
            .set_session_id(TEST_SESSION_ID);

        let client = ExpectClient::send(ProtoRequest::Activate(
            expected_activate_request_example_biome(is_background, is_auto_apply, &auto_apply),
        ))
        .successfully();

        let mut terrain = Terrain::example();
        terrain.set_auto_apply(auto_apply);

        super::handle(
            context,
            BiomeArg::Default,
            terrain,
            is_auto_apply,
            Some(client),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn spawns_shell_and_sends_activate_request_auto_apply_background() {
        let terrain_dir = Path::new(TEST_TERRAIN_DIR);
        let central_dir = Path::new(TEST_CENTRAL_DIR);

        let is_background = true;
        let auto_apply = AutoApply::Background;
        let is_auto_apply = true;

        let executor = ExpectZSH::with(MockExecutor::default(), terrain_dir)
            .get_fpath()
            .spawn_shell(
                expected_envs_with_activate_example_biome(is_auto_apply, &auto_apply),
                0,
                false,
                "".to_string(),
            )
            .successfully();

        let context = Context::build(terrain_dir, central_dir, false, executor)
            .set_session_id(TEST_SESSION_ID);

        let client = ExpectClient::send(ProtoRequest::Activate(
            expected_activate_request_example_biome(is_background, is_auto_apply, &auto_apply),
        ))
        .successfully();

        let mut terrain = Terrain::example();
        terrain.set_auto_apply(auto_apply);

        super::handle(
            context,
            BiomeArg::Default,
            terrain,
            is_auto_apply,
            Some(client),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn spawns_shell_and_sends_activate_request_auto_apply_replace() {
        let terrain_dir = Path::new(TEST_TERRAIN_DIR);
        let central_dir = Path::new(TEST_CENTRAL_DIR);

        let is_background = false;
        let auto_apply = AutoApply::Replace;
        let is_auto_apply = true;

        let executor = ExpectZSH::with(MockExecutor::default(), terrain_dir)
            .get_fpath()
            .spawn_shell(
                expected_envs_with_activate_example_biome(is_auto_apply, &auto_apply),
                0,
                false,
                "".to_string(),
            )
            .successfully();

        let context = Context::build(terrain_dir, central_dir, false, executor)
            .set_session_id(TEST_SESSION_ID);

        let client = ExpectClient::send(ProtoRequest::Activate(
            expected_activate_request_example_biome(is_background, is_auto_apply, &auto_apply),
        ))
        .successfully();

        let mut terrain = Terrain::example();
        terrain.set_auto_apply(auto_apply);

        super::handle(
            context,
            BiomeArg::Default,
            terrain,
            is_auto_apply,
            Some(client),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn spawns_shell_and_sends_activate_request_auto_apply_enabled() {
        let terrain_dir = Path::new(TEST_TERRAIN_DIR);
        let central_dir = Path::new(TEST_CENTRAL_DIR);

        let is_background = false;
        let auto_apply = AutoApply::Enabled;
        let is_auto_apply = true;

        let executor = ExpectZSH::with(MockExecutor::default(), terrain_dir)
            .get_fpath()
            .spawn_shell(
                expected_envs_with_activate_example_biome(is_auto_apply, &auto_apply),
                0,
                false,
                "".to_string(),
            )
            .successfully();

        let context = Context::build(terrain_dir, central_dir, false, executor)
            .set_session_id(TEST_SESSION_ID);

        let client = ExpectClient::send(ProtoRequest::Activate(
            expected_activate_request_example_biome(is_background, is_auto_apply, &auto_apply),
        ))
        .successfully();

        let mut terrain = Terrain::example();
        terrain.set_auto_apply(auto_apply);

        super::handle(
            context,
            BiomeArg::Default,
            terrain,
            is_auto_apply,
            Some(client),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn spawns_shell_and_sends_activate_request_auto_apply_off() {
        let terrain_dir = Path::new(TEST_TERRAIN_DIR);
        let central_dir = Path::new(TEST_CENTRAL_DIR);

        let is_background = false;
        let auto_apply = AutoApply::default();
        let is_auto_apply = true;

        let executor = ExpectZSH::with(MockExecutor::default(), terrain_dir)
            .get_fpath()
            .spawn_shell(
                expected_envs_with_activate_example_biome(is_auto_apply, &auto_apply),
                0,
                false,
                "".to_string(),
            )
            .successfully();

        let context = Context::build(terrain_dir, central_dir, false, executor)
            .set_session_id(TEST_SESSION_ID);

        let client = ExpectClient::send(ProtoRequest::Activate(
            expected_activate_request_example_biome(is_background, is_auto_apply, &auto_apply),
        ))
        .successfully();

        let mut terrain = Terrain::example();
        terrain.set_auto_apply(auto_apply);

        super::handle(
            context,
            BiomeArg::Default,
            terrain,
            is_auto_apply,
            Some(client),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn spawns_shell_and_sends_activate_request_example_biome() {
        let terrain_dir = Path::new(TEST_TERRAIN_DIR);
        let central_dir = Path::new(TEST_CENTRAL_DIR);

        let is_background = true;
        let auto_apply = AutoApply::default();
        let is_auto_apply = false;

        let executor = ExpectZSH::with(MockExecutor::default(), terrain_dir)
            .get_fpath()
            .spawn_shell(
                expected_envs_with_activate_example_biome(is_auto_apply, &auto_apply),
                0,
                false,
                "".to_string(),
            )
            .successfully();

        let context = Context::build(terrain_dir, central_dir, false, executor)
            .set_session_id(TEST_SESSION_ID);

        let client = ExpectClient::send(ProtoRequest::Activate(
            expected_activate_request_example_biome(is_background, is_auto_apply, &auto_apply),
        ))
        .successfully();

        let mut terrain = Terrain::example();
        terrain.set_auto_apply(auto_apply);

        super::handle(
            context,
            BiomeArg::Default,
            terrain,
            is_auto_apply,
            Some(client),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn spawns_shell_and_sends_activate_request_none() {
        let terrain_dir = Path::new(TEST_TERRAIN_DIR);
        let central_dir = Path::new(TEST_CENTRAL_DIR);

        let is_background = true;
        let auto_apply = AutoApply::All;
        let is_auto_apply = true;

        let executor = ExpectZSH::with(MockExecutor::default(), terrain_dir)
            .get_fpath()
            .spawn_shell(
                expected_envs_with_activate_none(is_auto_apply, &auto_apply),
                0,
                false,
                "".to_string(),
            )
            .successfully();

        let context = Context::build(terrain_dir, central_dir, false, executor)
            .set_session_id(TEST_SESSION_ID);

        let client = ExpectClient::send(ProtoRequest::Activate(expected_activate_request_none(
            is_background,
        )))
        .successfully();

        let mut terrain = Terrain::example();
        terrain.set_auto_apply(auto_apply);

        super::handle(
            context,
            BiomeArg::None,
            terrain,
            is_auto_apply,
            Some(client),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn spawns_shell_and_sends_activate_request_none_no_auto_apply() {
        let terrain_dir = Path::new(TEST_TERRAIN_DIR);
        let central_dir = Path::new(TEST_CENTRAL_DIR);

        let is_background = true;
        let auto_apply = AutoApply::All;
        let is_auto_apply = false;

        let executor = ExpectZSH::with(MockExecutor::default(), terrain_dir)
            .get_fpath()
            .spawn_shell(
                expected_envs_with_activate_none(is_auto_apply, &auto_apply),
                0,
                false,
                "".to_string(),
            )
            .successfully();

        let context = Context::build(terrain_dir, central_dir, false, executor)
            .set_session_id(TEST_SESSION_ID);

        let client = ExpectClient::send(ProtoRequest::Activate(expected_activate_request_none(
            is_background,
        )))
        .successfully();

        let mut terrain = Terrain::example();
        terrain.set_auto_apply(auto_apply);

        super::handle(
            context,
            BiomeArg::None,
            terrain,
            is_auto_apply,
            Some(client),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn spawns_shell_error() {
        let terrain_dir = Path::new(TEST_TERRAIN_DIR);
        let central_dir = Path::new(TEST_CENTRAL_DIR);

        let is_background = true;
        let auto_apply = AutoApply::default();
        let is_auto_apply = false;

        let executor = ExpectZSH::with(MockExecutor::default(), terrain_dir)
            .get_fpath()
            .spawn_shell(
                expected_envs_with_activate_example_biome(is_auto_apply, &auto_apply),
                -99,
                true,
                "error while spawning shell".to_string(),
            )
            .successfully();

        let context = Context::build(terrain_dir, central_dir, false, executor)
            .set_session_id(TEST_SESSION_ID);

        let client = ExpectClient::send(ProtoRequest::Activate(
            expected_activate_request_example_biome(is_background, is_auto_apply, &auto_apply),
        ))
        .successfully();

        let mut terrain = Terrain::example();
        terrain.set_auto_apply(auto_apply);

        let err = super::handle(
            context,
            BiomeArg::Default,
            terrain,
            is_auto_apply,
            Some(client),
        )
        .await
        .unwrap_err()
        .to_string();

        assert_eq!(
            err,
            "failed to spawn shell while entering terrain environment: failed to run zsh"
        );
    }

    #[tokio::test]
    async fn spawns_shell_non_zero_exit() {
        let terrain_dir = Path::new(TEST_TERRAIN_DIR);
        let central_dir = Path::new(TEST_CENTRAL_DIR);

        let is_background = true;
        let auto_apply = AutoApply::default();
        let is_auto_apply = false;

        let executor = ExpectZSH::with(MockExecutor::default(), terrain_dir)
            .get_fpath()
            .spawn_shell(
                expected_envs_with_activate_example_biome(is_auto_apply, &auto_apply),
                1,
                false,
                "".to_string(),
            )
            .successfully();

        let context = Context::build(terrain_dir, central_dir, false, executor)
            .set_session_id(TEST_SESSION_ID);

        let client = ExpectClient::send(ProtoRequest::Activate(
            expected_activate_request_example_biome(is_background, is_auto_apply, &auto_apply),
        ))
        .successfully();

        let mut terrain = Terrain::example();
        terrain.set_auto_apply(auto_apply);

        let err = super::handle(
            context,
            BiomeArg::Default,
            terrain,
            is_auto_apply,
            Some(client),
        )
        .await
        .unwrap_err()
        .to_string();

        assert_eq!(err, "spawned shell exited with code: Some(1)");
    }

    #[tokio::test]
    async fn send_request_error() {
        let terrain_dir = Path::new(TEST_TERRAIN_DIR);
        let central_dir = Path::new(TEST_CENTRAL_DIR);

        let is_background = true;
        let auto_apply = AutoApply::default();
        let is_auto_apply = false;

        let executor = ExpectZSH::with(MockExecutor::default(), terrain_dir)
            .get_fpath()
            .spawn_shell(
                expected_envs_with_activate_example_biome(is_auto_apply, &auto_apply),
                0,
                false,
                "".to_string(),
            )
            .successfully();

        let context = Context::build(terrain_dir, central_dir, false, executor)
            .set_session_id(TEST_SESSION_ID);

        let client = ExpectClient::send(ProtoRequest::Activate(
            expected_activate_request_example_biome(is_background, is_auto_apply, &auto_apply),
        ))
        .with_returning_error("failed to parse the request");

        let mut terrain = Terrain::example();
        terrain.set_auto_apply(auto_apply);

        let err = super::handle(
            context,
            BiomeArg::Default,
            terrain,
            is_auto_apply,
            Some(client),
        )
        .await
        .unwrap_err()
        .to_string();

        assert_eq!(err, "failed to spawn background processes while entering terrain environment: failed to parse the request");
    }
}
