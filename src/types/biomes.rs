use serde::{Deserialize, Serialize};

use super::{aliases::KeyValue, commands::Commands};

#[derive(Serialize, Deserialize, Debug)]
pub struct Biome {
    pub name: String,
    pub env: Option<KeyValue>,
    pub aliases: Option<KeyValue>,
    pub construct: Option<Commands>,
    pub deconstruct: Option<Commands>,
}
