use color_eyre::eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub(crate) mod about_file;
pub(crate) mod bin_file;
pub(crate) mod dotted_file;
pub(crate) mod plan;
pub(crate) mod runtime;
pub(crate) mod settings_file;

pub(crate) use about_file::{AboutEntry, AboutFile};
pub(crate) use bin_file::{BinFile, DownloadInstall, DownloadSpec};
pub(crate) use dotted_file::{ColorSection, ConfigSection, DottedFile, RepoConfig};
pub(crate) use plan::{Artifact, DownloadSource, Plan, PlannedDownload, PlannedFile};
pub(crate) use runtime::Runtime;
pub(crate) use settings_file::{IgnoreSection, Settings, SettingsFile};

pub(crate) const DOTTED_TOML: &str = "[dotted].toml";
pub(crate) const LOCAL_TOML: &str = "[local].toml";
pub(crate) const ARTIFACTS_DIR: &str = "[artifacts]";
pub(crate) const SETTINGS_DIR: &str = "[settings]";
pub(crate) const ABOUT_TOML: &str = "[about].toml";
pub(crate) const BIN_TOML: &str = "[bin].toml";
pub(crate) const FALLBACK_DEVICE: &str = "[device]";
pub(crate) const FALLBACK_USER: &str = "[user]";

pub(crate) const DEFAULT_DOTTED_DIR: &str = ".local/share/dotted";
pub(crate) const DEFAULT_BACKUP_DIR: &str = ".cache/dotted/backups";
pub(crate) const DEFAULT_ENV_SH: &str = "~/.config/dotted/env.sh";
pub(crate) const DEFAULT_GITIGNORE: &str = r"# Ignore everything
*
# Track only dotted control files
![dotted].toml
![settings]/
![settings]/**
![artifacts]/
![artifacts]/**
!.gitignore
";

pub(crate) fn read_toml<T>(path: &Path) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let content = fs::read_to_string(path).wrap_err_with(|| format!("read {}", path.display()))?;
    toml::from_str(&content).wrap_err_with(|| format!("parse {}", path.display()))
}

pub(crate) fn write_toml<T>(path: &Path, value: &T) -> Result<()>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).wrap_err_with(|| format!("create {}", parent.display()))?;
    }
    let content = toml::to_string_pretty(value)?;
    fs::write(path, content).wrap_err_with(|| format!("write {}", path.display()))
}
