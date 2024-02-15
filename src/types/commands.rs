use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Commands {
    pub foreground: Option<Vec<Command>>,
    pub background: Option<Vec<Command>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Command {
    pub exe: String,
    pub args: Option<Vec<String>>,
}

impl Commands {
    pub fn merge(&self, other: Self) -> Self {
        let foreground = get_merged_vecs(&self.foreground, &other.foreground);
        let background = get_merged_vecs(&self.background, &other.background);
        Commands {
            foreground,
            background,
        }
    }
}

fn get_merged_vecs(from: &Option<Vec<Command>>, to: &Option<Vec<Command>>) -> Option<Vec<Command>> {
    if from.is_none() && to.is_none() {
        return None;
    }
    if from.is_some() && !to.is_some() {
        return from.clone();
    }
    if to.is_some() && !from.is_some() {
        return to.clone();
    }

    let mut return_vec = to.clone().expect("to be present");
    return_vec.extend_from_slice(&from.clone().expect("to be present"));
    Some(return_vec)
}

pub fn get_merged_commands(from: &Option<Commands>, to: &Option<Commands>) -> Option<Commands> {
    if from.is_some() && !to.is_some() {
        return from.clone();
    }
    if to.is_some() && !from.is_some() {
        return to.clone();
    }

    if let Some(from) = &from {
        if let Some(to) = &to {
            return Some(to.merge(from.clone()));
        }
    }

    None
}

#[cfg(test)]
mod test {

}
