#[mockall_double::double]
use crate::common::execute::Executor;
use crate::daemon::types::config::DaemonConfig;
use crate::daemon::types::state_manager::StateManager;
use std::sync::Arc;

#[derive(Default, Clone, Debug)]
pub struct DaemonContext {
    is_root: bool,
    is_root_allowed: bool,
    executor: Arc<Executor>,
    state_manager: Arc<StateManager>,
}

impl DaemonContext {
    pub async fn new(
        is_root: bool,
        config: DaemonConfig,
        executor: Arc<Executor>,
        state_directory: &str,
    ) -> Self {
        let state_manager = StateManager::init(state_directory, config.history_size()).await;
        DaemonContext {
            is_root,
            is_root_allowed: config.is_root_allowed(),
            executor,
            state_manager: Arc::new(state_manager),
        }
    }

    pub fn state_manager(&self) -> Arc<StateManager> {
        self.state_manager.clone()
    }

    pub fn state_directory(&self) -> &str {
        self.state_manager.state_directory()
    }

    pub fn executor(&self) -> Arc<Executor> {
        self.executor.clone()
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
