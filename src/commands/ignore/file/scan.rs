/// CLI Command: `ignore file scan [--path <path>] [--filter <filter>]`
///
/// What it does:
/// Performs an unlimited-depth scan of the target directory to find and list all tracked, untracked, and ignored files.
///
/// Variations:
/// 1. `path` provided: Scans the specified directory path.
/// 2. Neither provided: Scans the current working directory.
/// 3. `--filter <filter>`: Filters by tracked/untracked/ignored/partial/masked status.
///
/// Decisions & Logic Branches:
/// - Acts as a convenience wrapper around `ignore file list`, configuring traversal depth to unlimited (`0`).
use color_eyre::eyre::Result;
use std::path::PathBuf;

use crate::cli::{FileFilter, LsArgs};
use crate::types::Runtime;

pub fn run(runtime: &Runtime, path: Option<PathBuf>, filter: Option<FileFilter>) -> Result<()> {
    let args = LsArgs {
        depth: Some(0), // 0 means unlimited
        path,
        filter,
    };
    crate::commands::ignore::file::list::run(runtime, &args)
}
