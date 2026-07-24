/// CLI Command: `backup list [timestamp] [--filter <path>]`
///
/// What it does:
/// Lists all available backups (by timestamp) or lists files stored inside a specific backup.
///
/// Variations:
/// 1. `timestamp` not provided: Lists all backup directory timestamps along with their parsed local date/time and relative age (e.g., "5m ago").
/// 2. `timestamp` provided: Lists all files backed up in that specific backup version.
/// 3. `--filter <path>` / `-f` provided (when `timestamp` is also provided): Filters the list of backed up files to only show those containing the filter path.
///
/// Decisions & Logic Branches:
/// - If a `timestamp` is specified, checks if the backup folder exists; if not, fails with an error.
/// - Performs a `WalkDir` over the backup directory to extract original file paths (by removing the backup root prefix).
/// - Format dates and times dynamically based on the current system local time zone.
use color_eyre::eyre::{Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::types::Runtime;
use crate::utils::style;

pub fn run(runtime: &Runtime, timestamp: Option<&str>, filter: Option<&Path>) -> Result<()> {
    crate::utils::print_banner("LIST BACKUPS", runtime);
    if let Some(ts) = timestamp {
        let root = runtime.backup_root().join(ts);
        if !root.exists() {
            bail!("backup timestamp not found: {ts}");
        }
        println!("Files in backup {ts}:");
        for entry in WalkDir::new(&root) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let relative = entry.path().strip_prefix(&root)?;
            let display_path = PathBuf::from("/").join(relative);
            let display_str = display_path.to_string_lossy();
            if filter.as_ref().is_some_and(|filter_path| {
                !display_str
                    .to_lowercase()
                    .contains(&filter_path.to_string_lossy().to_lowercase())
            }) {
                continue;
            }
            println!("  /{}", relative.display());
        }
    } else {
        if !runtime.backup_root().exists() {
            println!("No backups found.");
            return Ok(());
        }
        for entry in fs::read_dir(runtime.backup_root())? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if let Ok(ts_val) = name.parse::<i64>() {
                let dt = chrono::DateTime::from_timestamp(ts_val, 0)
                    .map(|d| d.with_timezone(&chrono::Local))
                    .map_or_else(
                        || "invalid time".to_string(),
                        |d| d.format("%Y-%m-%d %H:%M:%S").to_string(),
                    );
                let dur = chrono::Utc::now().timestamp() - ts_val;
                let ago = if dur < 60 {
                    format!("{dur}s ago")
                } else if dur < 3600 {
                    format!("{}m ago", dur / 60)
                } else if dur < 86400 {
                    format!("{}h ago", dur / 3600)
                } else {
                    format!("{}d ago", dur / 86400)
                };
                println!("{}  ({dt}, {ago})", style(&name, "36", runtime));
            } else {
                println!("{name}");
            }
        }
    }
    Ok(())
}
