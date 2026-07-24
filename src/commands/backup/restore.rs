/// CLI Command: `backup restore <timestamp> [path]`
///
/// What it does:
/// Restores backed up files from a specific backup version back to their original target locations on the filesystem.
///
/// Variations:
/// 1. `path` provided: Restores only the single specified file from the backup version.
/// 2. `path` not provided: Restores all files contained in the backup snapshot.
///
/// Decisions & Logic Branches:
/// - Fails if the backup snapshot directory matching the `timestamp` does not exist.
/// - Resolves destination target paths relative to the current system environment.
/// - Restores files using `restore_one`, which copies the file content back, creates any required parent directories, and preserves permissions.
use color_eyre::eyre::{Result, bail};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::commands::lib::restore_one;
use crate::types::Runtime;

pub fn run(runtime: &Runtime, timestamp: &str, path: Option<&Path>) -> Result<()> {
    crate::utils::print_banner("RESTORE BACKUP", runtime);
    let root = runtime.backup_root().join(timestamp);
    if !root.exists() {
        bail!("backup timestamp not found: {timestamp}");
    }

    if let Some(single) = path {
        let relative = single.strip_prefix("/").unwrap_or(single);
        let source = root.join(relative);
        let target = runtime.resolve_abs_target(single);
        restore_one(&source, &target)?;
    } else {
        for entry in WalkDir::new(&root) {
            let entry = entry?;
            if !entry.file_type().is_file() {
                continue;
            }
            let relative = entry.path().strip_prefix(&root)?;
            let target = runtime.resolve_abs_target(&PathBuf::from("/").join(relative));
            restore_one(entry.path(), &target)?;
        }
    }
    Ok(())
}
