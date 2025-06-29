use std::fmt::Display;

const AUTO_APPLY_ENABLED: &str = "enabled";
const AUTO_APPLY_BACKGROUND: &str = "background";
const AUTO_APPLY_REPLACE: &str = "replace";
const AUTO_APPLY_ALL: &str = "all";
const AUTO_APPLY_OFF: &str = "off";

#[derive(Default)]
pub enum AutoApply {
    All,
    Background,
    Replace,
    Enabled,
    #[default]
    Off,
}

impl Display for AutoApply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match &self {
            AutoApply::All => AUTO_APPLY_ALL,
            AutoApply::Background => AUTO_APPLY_BACKGROUND,
            AutoApply::Replace => AUTO_APPLY_REPLACE,
            AutoApply::Enabled => AUTO_APPLY_ENABLED,
            AutoApply::Off => AUTO_APPLY_OFF,
        };
        write!(f, "{value}")
    }
}
