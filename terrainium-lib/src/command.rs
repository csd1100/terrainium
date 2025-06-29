use std::path::PathBuf;

pub struct Command {
    exe: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
}
impl Command {
    /// creates a new command object for Executor to use
    pub fn new(exe: String, args: Vec<String>, cwd: Option<PathBuf>) -> Self {
        Command { exe, args, cwd }
    }

    /// set arguments for command to be executed
    pub fn set_args(&mut self, args: Vec<String>) {
        self.args = args;
    }
}

impl From<Command> for std::process::Command {
    fn from(value: Command) -> std::process::Command {
        let mut command = std::process::Command::new(value.exe);
        command
            .args(value.args)
            .current_dir(value.cwd.expect("cwd to be present"));
        command
    }
}
