use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Commands {
    pub exec: Vec<Command>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Command {
    pub exe: String,
    pub args: Option<Vec<String>>,
}

fn get_merged_vecs(from: &Vec<Command>, to: &Vec<Command>) -> Vec<Command> {
    let mut return_vec = to.clone();
    return_vec.extend_from_slice(&from);
    return return_vec;
}

impl Commands {
    pub fn merge(&self, other: Self) -> Self {
        let execs = get_merged_vecs(&self.exec, &other.exec);
        return Commands { exec: execs };
    }
}
