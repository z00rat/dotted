/// CLI Command: `files list [--path <path>] [--filter <tracked|untracked|ignored|mixed>]`
///
/// Lists files below the current directory (or `--path`) with status-aware
/// classification ([tracked], [untracked], [ignored], [mixed]) shared by scan and ignore selection.
use color_eyre::eyre::Result;
use std::collections::BTreeSet;
use std::env;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::commands::lib::{is_ignored_dir, matches_any_glob};
use crate::plan::build_plan;
use crate::types::Runtime;
use crate::utils::style;

pub fn run(runtime: &Runtime, args: &crate::cli::LsArgs) -> Result<()> {
    let plan = build_plan(runtime, None)?;
    let root = args.path.clone().unwrap_or(env::current_dir()?);
    let max_depth = args.depth.unwrap_or(1);
    let tracked: BTreeSet<PathBuf> = plan
        .files
        .iter()
        .map(|file| file.display_target.clone())
        .collect();
    let show = |status: &str| args.filter.as_deref().is_none_or(|filter| filter == status);
    let has_tracked_files_under = |dir: &Path| tracked.iter().any(|file| file.starts_with(dir));
    let mut it = WalkDir::new(&root)
        .max_depth(if max_depth == 0 {
            usize::MAX
        } else {
            max_depth
        })
        .into_iter();

    while let Some(Ok(entry)) = it.next() {
        if entry.path() == root {
            continue;
        }
        let path = entry.path().to_path_buf();
        let display = runtime.display_path(&path);
        let display_str = if entry.file_type().is_dir() {
            format!("{}/", display.display())
        } else {
            display.display().to_string()
        };

        let is_ign = is_ignored_dir(&entry, &plan.ignored_folders)
            || (entry.file_type().is_file() && matches_any_glob(&path, &plan.ignored_files));

        let status = if is_ign {
            "ignored"
        } else if entry.file_type().is_dir() {
            let has_tracked = has_tracked_files_under(&path);
            let total_files = WalkDir::new(&path)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_file())
                .count();
            let tracked_count = tracked
                .iter()
                .filter(|file| file.starts_with(&path))
                .count();
            if tracked_count > 0 && tracked_count < total_files {
                "mixed"
            } else if has_tracked {
                "tracked"
            } else {
                "untracked"
            }
        } else if tracked.contains(&path) {
            "tracked"
        } else {
            "untracked"
        };

        if show(status) {
            let color = match status {
                "tracked" => "32",
                "mixed" => "36",
                "ignored" => "90",
                _ => "33",
            };
            let bracketed = format!("[{status}]");
            println!(
                "{} {}",
                style(&format!("{bracketed:<11}"), color, runtime),
                display_str
            );
        }
        if entry.file_type().is_dir()
            && (status == "untracked" || is_ignored_dir(&entry, &plan.ignored_folders))
        {
            it.skip_current_dir();
        }
    }
    Ok(())
}
