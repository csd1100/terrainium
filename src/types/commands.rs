use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub enum Commands {
    #[serde(rename = "exec")]
    Exec(Option<Vec<String>>),
}
