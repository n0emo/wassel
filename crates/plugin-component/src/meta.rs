use std::{collections::HashMap, path::PathBuf, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMeta {
    pub id: String,

    #[serde(default = "String::default")]
    pub name: String,

    #[serde(default = "default_version")]
    pub version: String,

    #[serde(default = "Option::default")]
    pub description: Option<String>,

    #[serde(default = "HashMap::default")]
    pub variables: HashMap<String, String>,

    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,

    #[serde(default = "default_endpoint")]
    pub endpoint: String,
}

fn default_version() -> String {
    "0.0.0".to_owned()
}

fn default_data_dir() -> PathBuf {
    PathBuf::from_str("data").expect("`data` should be valid path")
}

fn default_endpoint() -> String {
    "/".to_owned()
}
