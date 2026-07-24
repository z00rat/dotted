use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub(crate) struct AboutFile {
    #[serde(default)]
    pub(crate) about: BTreeMap<String, AboutEntry>,
    #[serde(default)]
    pub(crate) maintainer: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct AboutEntry {
    #[serde(default)]
    pub(crate) r: i64,
    #[serde(default)]
    pub(crate) description: String,
}
