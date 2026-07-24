use crate::types::{BinFile, DownloadInstall};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub(crate) struct Artifact {
    pub(crate) id: String,
    pub(crate) repo: String,
    pub(crate) name: String,
    pub(crate) dir: PathBuf,
    pub(crate) revision: i64,
    pub(crate) description: String,
    pub(crate) bin: BinFile,
}

#[derive(Clone, Debug)]
pub(crate) struct PlannedFile {
    pub(crate) artifact_id: String,
    pub(crate) source: PathBuf,
    pub(crate) target: PathBuf,
    pub(crate) display_target: PathBuf,
    pub(crate) text: Option<String>,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct Plan {
    pub(crate) artifacts: Vec<Artifact>,
    pub(crate) files: Vec<PlannedFile>,
    pub(crate) env: BTreeMap<String, String>,
    pub(crate) env_overrides: Vec<String>,
    pub(crate) packages: BTreeMap<String, BTreeSet<String>>,
    pub(crate) flatpaks: BTreeSet<String>,
    pub(crate) downloads: Vec<PlannedDownload>,
    pub(crate) ignored_folders: BTreeSet<PathBuf>,
    pub(crate) ignored_files: BTreeSet<PathBuf>,
}

#[derive(Clone, Debug)]
pub(crate) struct PlannedDownload {
    pub(crate) artifact_id: String,
    pub(crate) source: DownloadSource,
    pub(crate) install: DownloadInstall,
    pub(crate) install_path: PathBuf,
    pub(crate) display_path: PathBuf,
}

#[derive(Clone, Debug)]
pub(crate) enum DownloadSource {
    Url(String),
    Zip { url: String, path: String },
}

impl PlannedDownload {
    pub(crate) fn url_or_zip_url(&self) -> &str {
        match &self.source {
            DownloadSource::Url(url) | DownloadSource::Zip { url, .. } => url.as_str(),
        }
    }
}
