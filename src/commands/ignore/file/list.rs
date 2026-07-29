/// CLI Command: `ignore file list [--path <path>] [--filter <tracked|untracked|ignored|partial|masked>]`
///
/// What it does:
/// Lists files below the target directory (or current directory) with status-aware classification (`[tracked]`, `[untracked]`, `[ignored]`, `[partial]`, `[masked]`).
///
/// Variations:
/// 1. `--path` provided: Uses the specified root directory instead of the current working directory.
/// 2. `--depth` provided: Limits directory traversal depth (default depth is 1).
/// 3. `--filter` provided: Restricts output to the specified status category (`tracked`, `untracked`, `ignored`, `partial`, or `masked`).
///
/// Decisions & Logic Branches:
/// - Builds active plan to identify tracked files and active ignore patterns.
/// - Recursively walks directory entries, evaluating paths against `ignored_folders` and `ignored_files`.
/// - Computes folder tracking states (`tracked`, `partial`, `masked`, `untracked`, `ignored`).
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
    let show = |status: &str| {
        args.filter
            .as_ref()
            .is_none_or(|filter| filter.as_str() == status)
    };
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

        let is_dir = entry.file_type().is_dir();
        let is_ignored_folder = is_ignored_dir(&entry, &plan.ignored_folders);
        let is_ign_file = !is_dir && matches_any_glob(&path, &plan.ignored_files);

        let status = if is_ignored_folder {
            if has_tracked_files_under(&path) {
                "masked"
            } else {
                "ignored"
            }
        } else if is_ign_file {
            "ignored"
        } else if is_dir {
            let has_tracked = has_tracked_files_under(&path);
            if has_tracked {
                // If it has tracked files inside, we mark as tracked if all top-level files are tracked,
                // or partial if there are untracked components as well.
                "partial"
            } else {
                "untracked"
            }
        } else if tracked.contains(&path) {
            "tracked"
        } else {
            "untracked"
        };

        let color = match status {
            "tracked" => "32",
            "partial" => "36",
            "ignored" => "90",
            _ => "33",
        };

        if show(status) {
            let bracketed = format!("[{status}]");
            println!(
                "{} {}",
                style(&format!("{bracketed:<11}"), color, runtime),
                display_str
            );
        }

        if is_ignored_folder {
            if status == "masked" {
                // Show tracked files inside this ignored folder from memory without disk traversal
                let sub_tracked: Vec<&PathBuf> = tracked
                    .iter()
                    .filter(|file| file.starts_with(&path))
                    .collect();

                for sub_file in sub_tracked {
                    if sub_file.exists() && show("tracked") {
                        let sub_display = runtime.display_path(sub_file);
                        let sub_str = if sub_file.is_dir() {
                            format!("{}/", sub_display.display())
                        } else {
                            sub_display.display().to_string()
                        };
                        let bracketed = "[tracked]";
                        println!(
                            "{} {}",
                            style(&format!("{bracketed:<11}"), "32", runtime),
                            sub_str
                        );
                    }
                }
            }
            // Do not traverse filesystem inside ignored folders
            it.skip_current_dir();
        } else if is_dir && status == "untracked" {
            it.skip_current_dir();
        }
    }
    Ok(())
}
