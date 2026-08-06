/// CLI Command: `ignore file list [--path <path>] [--filter <tracked|untracked|ignored|partial|masked>]`
///
/// What it does:
/// Lists files below the target directory (or current directory) in flat status-padded format with classification (`[tracked]`, `[untracked]`, `[ignored]`, `[partial]`, `[masked]`).
///
/// Variations:
/// 1. `--path` provided: Uses the specified root directory instead of the current working directory.
/// 2. `--depth` provided: Limits directory traversal depth (default depth is 1).
/// 3. `--filter` provided: Restricts output to the specified status category.
///
/// Decisions & Logic Branches:
/// - Evaluates paths against active plan to resolve tracking and ignore states.
/// - Formats path strings relative to the target root directory (`strip_prefix`).
/// - Resolves status colors dynamically from `[dotted].toml` settings via `status_color`.
/// - Sorts directory entries case-insensitively with folders listed before files (`cmp_walkdir_entries`).
use color_eyre::eyre::Result;
use std::collections::BTreeSet;
use std::env;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::commands::lib::matches_any_glob;
use crate::plan::build_plan;
use crate::status_color::status_color;
use crate::types::{Plan, Runtime};
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
    let has_tracked = |dir: &Path| tracked.iter().any(|file| file.starts_with(dir));

    let mut it = WalkDir::new(&root)
        .max_depth(if max_depth == 0 {
            usize::MAX
        } else {
            max_depth
        })
        .sort_by(crate::utils::cmp_walkdir_entries)
        .into_iter();

    while let Some(Ok(entry)) = it.next() {
        if entry.path() == root {
            continue;
        }
        let path = entry.path().to_path_buf();
        let is_dir = path.is_dir();
        let rel_path = path.strip_prefix(&root).unwrap_or(&path);
        let display_str = if is_dir {
            format!("{}/", rel_path.display())
        } else {
            rel_path.display().to_string()
        };

        let is_ignored_folder = is_dir && plan.ignored_folders.contains(&path);
        let is_ign_file = !is_dir && matches_any_glob(&path, &plan.ignored_files);

        let status = classify_item_status(
            &path,
            is_dir,
            is_ignored_folder,
            is_ign_file,
            &tracked,
            &plan,
            has_tracked,
        );

        if show(status) {
            print_item_line(status, &display_str, runtime);
        }

        if is_ignored_folder && status == "masked" {
            print_masked_subfiles(&path, &root, &tracked, &show, runtime);
        }

        if entry.file_type().is_dir() && (is_ignored_folder || status == "untracked") {
            it.skip_current_dir();
        }
    }

    Ok(())
}

fn classify_item_status(
    path: &Path,
    is_dir: bool,
    is_ignored_folder: bool,
    is_ign_file: bool,
    tracked: &BTreeSet<PathBuf>,
    plan: &Plan,
    has_tracked: impl Fn(&Path) -> bool,
) -> &'static str {
    if is_ignored_folder {
        if has_tracked(path) {
            "masked"
        } else {
            "ignored"
        }
    } else if is_ign_file {
        "ignored"
    } else if is_dir {
        if has_tracked(path) {
            let dir_entries = std::fs::read_dir(path)
                .map(|rd| rd.filter_map(Result::ok).collect::<Vec<_>>())
                .unwrap_or_default();
            let all_tracked = !dir_entries.is_empty()
                && dir_entries.iter().all(|e| {
                    let p = e.path();
                    let e_is_dir = e.file_type().is_ok_and(|ft| ft.is_dir());
                    if e_is_dir {
                        has_tracked(&p)
                    } else {
                        tracked.contains(&p)
                    }
                });
            if all_tracked { "tracked" } else { "partial" }
        } else if crate::ignore::is_dir_all_ignored(path, plan) {
            "ignored"
        } else {
            "untracked"
        }
    } else if tracked.contains(path) {
        "tracked"
    } else {
        "untracked"
    }
}

fn print_item_line(status: &str, display_str: &str, runtime: &Runtime) {
    let color = status_color(status, runtime);
    let bracketed = format!("[{status}]");
    println!(
        "{} {}",
        style(&format!("{bracketed:<11}"), &color, runtime),
        display_str
    );
}

fn print_masked_subfiles(
    folder: &Path,
    root: &Path,
    tracked: &BTreeSet<PathBuf>,
    show: &impl Fn(&str) -> bool,
    runtime: &Runtime,
) {
    let sub_tracked: Vec<&PathBuf> = tracked
        .iter()
        .filter(|file| file.starts_with(folder))
        .collect();

    for sub_file in sub_tracked {
        if sub_file.exists() && show("tracked") {
            let rel_sub = sub_file.strip_prefix(root).unwrap_or(sub_file);
            let sub_str = if sub_file.is_dir() {
                format!("{}/", rel_sub.display())
            } else {
                rel_sub.display().to_string()
            };
            print_item_line("tracked", &sub_str, runtime);
        }
    }
}
