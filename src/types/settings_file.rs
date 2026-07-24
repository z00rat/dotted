use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub(crate) struct SettingsFile {
    #[serde(default)]
    pub(crate) artifacts: ArtifactsSection,
    #[serde(default)]
    pub(crate) ignore: IgnoreSection,
    #[serde(default)]
    pub(crate) replace: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) env: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub(crate) struct ArtifactsSection {
    #[serde(default)]
    pub(crate) enable: Vec<String>,
    #[serde(default)]
    pub(crate) disable: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub(crate) struct IgnoreSection {
    #[serde(default)]
    pub(crate) folder: Vec<String>,
    #[serde(default)]
    pub(crate) file: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub(crate) struct Settings {
    pub(crate) enable: BTreeSet<String>,
    pub(crate) disable: BTreeSet<String>,
    pub(crate) ignore_folders: BTreeSet<String>,
    pub(crate) ignore_files: BTreeSet<String>,
    pub(crate) replace: BTreeMap<String, String>,
    pub(crate) env: BTreeMap<String, String>,
}

impl Settings {
    pub(crate) fn empty() -> Self {
        Self {
            enable: BTreeSet::new(),
            disable: BTreeSet::new(),
            ignore_folders: BTreeSet::new(),
            ignore_files: BTreeSet::new(),
            replace: BTreeMap::new(),
            env: BTreeMap::new(),
        }
    }

    pub(crate) fn merge_file(&mut self, file: SettingsFile) {
        self.enable.extend(file.artifacts.enable);
        self.disable.extend(file.artifacts.disable);
        self.ignore_folders.extend(file.ignore.folder);
        self.ignore_files.extend(file.ignore.file);
        self.replace.extend(file.replace);
        self.env.extend(file.env);
    }
}
