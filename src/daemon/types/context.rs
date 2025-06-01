use crate::daemon::types::state_manager::StateManager;

#[derive(Default, Clone, Debug)]
pub struct DaemonContext {
    is_root: bool,
    is_root_allowed: bool,
    state_manager: StateManager,
}

impl DaemonContext {
    pub async fn new(is_root: bool, is_root_allowed: bool) -> Self {
        DaemonContext {
            is_root,
            is_root_allowed,
            state_manager: StateManager::init().await,
        }
    }

    pub fn state_manager(&self) -> &StateManager {
        &self.state_manager
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
