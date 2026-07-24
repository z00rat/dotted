/// CLI Command: `files scan [--path <path>] [--filter <filter>]`
///
/// What it does:
/// Performs an unlimited-depth scan of the target directory to find and list all tracked, untracked, and ignored files.
///
/// Variations:
/// 1. `path` provided: Scans the specified directory path.
/// 2. Neither provided: Scans the current working directory.
/// 3. `--filter <filter>` / `-f`: Filters by tracked/untracked/ignored status.
///
/// Decisions & Logic Branches:
/// - Simply acts as a wrapper around the `files list` command, overriding the search depth parameter to `0` (which triggers recursive, unlimited-depth scanning).
use color_eyre::eyre::Result;
use std::path::PathBuf;

use crate::cli::LsArgs;
use crate::types::Runtime;

pub fn run(runtime: &Runtime, path: Option<PathBuf>, filter: Option<String>) -> Result<()> {
    let args = LsArgs {
        depth: Some(0), // 0 means unlimited
        path,
        filter,
    };
    crate::commands::files::list::run(runtime, &args)
}
