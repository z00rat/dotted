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
pub(crate) const AGENTS_MD: &str = "AGENTS.md";
pub(crate) const MEMORY_MD: &str = "memory.md";
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
!\[dotted\].toml
!\[settings\]/
!\[settings\]/**
!\[artifacts\]/
!\[artifacts\]/**
!AGENTS.md
!memory.md
!.gitignore
";

pub(crate) const DEFAULT_AGENTS_MD: &str = include_str!("../templates/AGENTS.md");
pub(crate) const DEFAULT_MEMORY_MD: &str = include_str!("../templates/memory.md");

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

pub(crate) fn sort_toml_value(value: &mut toml::Value) {
    match value {
        toml::Value::Array(arr) => {
            for item in arr.iter_mut() {
                sort_toml_value(item);
            }
            arr.sort_by(|a, b| match (a, b) {
                (toml::Value::String(s1), toml::Value::String(s2)) => s1.cmp(s2),
                (toml::Value::Integer(i1), toml::Value::Integer(i2)) => i1.cmp(i2),
                (toml::Value::Float(f1), toml::Value::Float(f2)) => {
                    f1.partial_cmp(f2).unwrap_or(std::cmp::Ordering::Equal)
                }
                (toml::Value::Boolean(b1), toml::Value::Boolean(b2)) => b1.cmp(b2),
                (toml::Value::Datetime(d1), toml::Value::Datetime(d2)) => {
                    d1.to_string().cmp(&d2.to_string())
                }
                (toml::Value::Table(t1), toml::Value::Table(t2)) => {
                    let name1 = t1.get("name").and_then(|v| v.as_str());
                    let name2 = t2.get("name").and_then(|v| v.as_str());
                    match (name1, name2) {
                        (Some(n1), Some(n2)) => n1.cmp(n2),
                        _ => t1.to_string().cmp(&t2.to_string()),
                    }
                }
                _ => std::cmp::Ordering::Equal,
            });
        }
        toml::Value::Table(table) => {
            for (_, v) in table.iter_mut() {
                sort_toml_value(v);
            }
        }
        _ => {}
    }
}
