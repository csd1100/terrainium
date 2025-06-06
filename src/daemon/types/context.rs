use crate::daemon::types::config::DaemonConfig;
use crate::daemon::types::state_manager::StateManager;
use std::sync::Arc;

#[derive(Default, Clone, Debug)]
pub struct DaemonContext {
    is_root: bool,
    is_root_allowed: bool,
    state_manager: Arc<StateManager>,
}

impl DaemonContext {
    pub async fn new(config: DaemonConfig, is_root: bool) -> Self {
        let state_manager = StateManager::init(config.history_size()).await;
        DaemonContext {
            is_root,
            is_root_allowed: config.is_root_allowed(),
            state_manager: Arc::new(state_manager),
        }
    }

    pub fn state_manager(&self) -> Arc<StateManager> {
        self.state_manager.clone()
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
