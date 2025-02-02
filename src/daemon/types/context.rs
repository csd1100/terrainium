#[derive(Default, Clone, Debug, PartialEq)]
pub struct DaemonContext {
    is_root: bool,
    is_root_allowed: bool,
}

impl DaemonContext {
    pub fn new(is_root: bool, is_root_allowed: bool) -> Self {
        DaemonContext {
            is_root,
            is_root_allowed,
        }
    }

    pub fn should_early_exit(&self) -> bool {
        self.is_root && !self.is_root_allowed
    }

    pub fn should_run_sudo(&self) -> bool {
        self.is_root && self.is_root_allowed
    }
}
