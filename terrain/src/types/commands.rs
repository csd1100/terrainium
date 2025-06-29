use terrainium_lib::command::Command;

/// Stores foreground and background [Command]s to be run
pub struct Commands {
    foreground: Vec<Command>,
    background: Vec<Command>,
}

impl Commands {
    /// Constructs new [Commands]
    pub fn new(foreground: Vec<Command>, background: Vec<Command>) -> Self {
        Self {
            foreground,
            background,
        }
    }

    /// Appends other [Commands] with self
    pub(super) fn append(&mut self, other: Commands) {
        self.foreground.extend(other.foreground);
        self.background.extend(other.background);
    }
}
