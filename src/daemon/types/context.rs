#[mockall_double::double]
use crate::common::execute::Executor;
use crate::common::types::paths::DaemonPaths;
use crate::daemon::types::config::DaemonConfig;
use crate::daemon::types::state_manager::StateManager;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[derive(Default, Clone, Debug)]
pub struct DaemonContext {
    is_root: bool,
    is_root_allowed: bool,
    executor: Arc<Executor>,
    cancellation_token: CancellationToken,
    state_manager: Arc<StateManager>,
}

impl DaemonContext {
    pub async fn new(
        is_root: bool,
        config: DaemonConfig,
        executor: Arc<Executor>,
        cancellation_token: CancellationToken,
        daemon_paths: DaemonPaths,
    ) -> Self {
        let state_manager = StateManager::init(daemon_paths, config.history_size()).await;
        DaemonContext {
            is_root,
            is_root_allowed: config.is_root_allowed(),
            executor,
            cancellation_token,
            state_manager: Arc::new(state_manager),
        }
    }

    pub fn state_manager(&self) -> Arc<StateManager> {
        self.state_manager.clone()
    }

    pub fn state_paths(&self) -> &DaemonPaths {
        self.state_manager.state_paths()
    }

    pub fn executor(&self) -> Arc<Executor> {
        self.executor.clone()
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }

    pub fn setup_state_manager(&self) {
        self.state_manager.setup_cleanup();
    }

    pub fn should_exit_early(&self) -> bool {
        self.is_root && !self.is_root_allowed
    }

    pub fn should_run_sudo(&self) -> bool {
        self.is_root && self.is_root_allowed
    }
}
