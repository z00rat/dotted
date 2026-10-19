/// CLI Command: `backup list [timestamp] [--filter <path>]`
///
/// What it does:
/// Lists all available backups (by timestamp) or paths stored inside a specific backup.
///
/// Variations:
/// 1. `timestamp` not provided: Lists all backup directory timestamps along with their parsed local date/time and relative age (e.g., "5m ago").
/// 2. `timestamp` provided: Lists all paths backed up in that specific version.
/// 3. `--filter <path>` / `-f` provided (when `timestamp` is also provided): Filters the listed paths to those containing the filter path.
///
/// Decisions & Logic Branches:
/// - If a `timestamp` is specified, checks if the backup folder exists; if not, fails with an error.
/// - Performs a `WalkDir` over the backup directory to extract original file paths (by removing the backup root prefix).
/// - Format dates and times dynamically based on the current system local time zone.
use color_eyre::eyre::{Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::cli::OutputFormat;
use crate::types::Runtime;
use crate::utils::style;

fn snapshot_paths(root: &Path, filter: Option<&Path>) -> Result<Vec<(PathBuf, String)>> {
    let mut paths = Vec::new();
    for entry in WalkDir::new(root).sort_by(crate::utils::cmp_walkdir_entries) {
        let entry = entry?;
        if !entry.file_type().is_file() && !entry.file_type().is_symlink() {
            continue;
        }
        let relative = entry.path().strip_prefix(root)?.to_path_buf();
        let display = PathBuf::from("/")
            .join(&relative)
            .to_string_lossy()
            .into_owned();
        if filter.is_some_and(|filter_path| {
            !display
                .to_lowercase()
                .contains(&filter_path.to_string_lossy().to_lowercase())
        }) {
            continue;
        }
        paths.push((relative, display));
    }
    Ok(paths)
}

fn list_snapshot_paths(
    root: &Path,
    timestamp: &str,
    filter: Option<&Path>,
    format: &OutputFormat,
) -> Result<()> {
    let paths = snapshot_paths(root, filter)?;
    if !matches!(format, OutputFormat::Human) {
        let display_paths: Vec<_> = paths.iter().map(|(_, display)| display).collect();
        return crate::utils::output::print_structured(
            format,
            &serde_json::json!({"snapshot": timestamp, "paths": display_paths}),
        );
    }
    println!("Paths in backup {timestamp}:");
    for (relative, _) in paths {
        println!("  /{}", relative.display());
    }
    Ok(())
}

fn snapshot_info(name: &str) -> serde_json::Value {
    let parsed = name.parse::<i64>().ok();
    let created_at = parsed
        .and_then(|timestamp| chrono::DateTime::from_timestamp(timestamp, 0))
        .map(|date| {
            date.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_default();
    let age = parsed
        .map(|timestamp| {
            let duration = chrono::Utc::now().timestamp() - timestamp;
            if duration < 60 {
                format!("{duration}s")
            } else if duration < 3600 {
                format!("{}m", duration / 60)
            } else if duration < 86400 {
                format!("{}h", duration / 3600)
            } else {
                format!("{}d", duration / 86400)
            }
        })
        .unwrap_or_default();
    serde_json::json!({"id": name, "created_at": created_at, "age": age})
}

fn snapshot_entries(runtime: &Runtime) -> Result<Vec<fs::DirEntry>> {
    let mut entries = fs::read_dir(runtime.backup_root())?
        .filter_map(std::result::Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by(crate::utils::cmp_dir_entries);
    Ok(entries)
}

fn list_snapshots(runtime: &Runtime, format: &OutputFormat) -> Result<()> {
    if !runtime.backup_root().exists() {
        if !matches!(format, OutputFormat::Human) {
            return crate::utils::output::print_structured(
                format,
                &serde_json::json!({"snapshots": []}),
            );
        }
        println!("No backups found.");
        return Ok(());
    }
    let entries = snapshot_entries(runtime)?;
    if !matches!(format, OutputFormat::Human) {
        let snapshots: Vec<_> = entries
            .iter()
            .map(|entry| snapshot_info(&entry.file_name().to_string_lossy()))
            .collect();
        return crate::utils::output::print_structured(
            format,
            &serde_json::json!({"snapshots": snapshots}),
        );
    }
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Ok(timestamp) = name.parse::<i64>() {
            let date = chrono::DateTime::from_timestamp(timestamp, 0)
                .map(|date| date.with_timezone(&chrono::Local))
                .map_or_else(
                    || "invalid time".to_string(),
                    |date| date.format("%Y-%m-%d %H:%M:%S").to_string(),
                );
            let age = snapshot_info(&name)["age"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            println!("{}  ({date}, {age} ago)", style(&name, "36", runtime));
        } else {
            println!("{name}");
        }
    }
    Ok(())
}

pub fn run(
    runtime: &Runtime,
    timestamp: Option<&str>,
    filter: Option<&Path>,
    format: &OutputFormat,
) -> Result<()> {
    if let Some(timestamp) = timestamp {
        let root = runtime.backup_root().join(timestamp);
        if !root.exists() {
            bail!("backup timestamp not found: {timestamp}");
        }
        return list_snapshot_paths(&root, timestamp, filter, format);
    }
    list_snapshots(runtime, format)
}
