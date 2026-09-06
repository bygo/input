use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Analysis {
    pub sentences: Vec<String>,

    #[serde(default)]
    pub native: Option<String>,
    #[serde(default)]
    pub source: String,
}
