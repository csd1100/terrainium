use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Commands {
    pub exec: Vec<Command>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Command {
    pub exe: String,
    pub args: Option<Vec<String>>,
}
