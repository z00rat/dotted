use crate::types::IgnoreSection;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub(crate) struct BinFile {
    #[serde(default)]
    pub(crate) download: BTreeMap<String, DownloadSpec>,
    #[serde(default)]
    pub(crate) distro: BTreeMap<String, PackageSet>,
    #[serde(default)]
    pub(crate) flatpak: PackageSet,
    #[serde(default)]
    pub(crate) env: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) ignore: IgnoreSection,
    #[serde(default)]
    pub(crate) config: BinConfig,
    #[serde(flatten)]
    pub(crate) extra: BTreeMap<String, toml::Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub(crate) struct BinConfig {
    #[serde(default)]
    pub(crate) remove: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub(crate) struct PackageSet {
    #[serde(default)]
    pub(crate) packages: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct DownloadSpec {
    pub(crate) url: Option<String>,
    pub(crate) zip: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) hash: Option<String>,
    pub(crate) install: DownloadInstall,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DownloadInstall {
    Local,
    System,
}
