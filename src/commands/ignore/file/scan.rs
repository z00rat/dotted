/// CLI Command: `ignore file scan [--path <path>] [--filter <filter>]`
///
/// What it does:
/// Performs an unlimited-depth scan of the target directory to build and display a hierarchical tree
/// of tracked, untracked, partial, ignored, and masked files.
///
/// Variations:
/// 1. `path` provided: Scans the specified directory path.
/// 2. Neither provided: Scans the current working directory.
/// 3. `--filter <filter>`: Filters by tracked/untracked/ignored/partial/masked status.
///
/// Decisions & Logic Branches:
/// - Uses `termtree::Tree` to output a structured visual tree with relative basenames for sub-nodes.
/// - Resolves status colors dynamically from `[dotted].toml` configuration via `status_color`.
/// - Sorts directory contents case-insensitively with directories placed before files (`cmp_dir_entries`).
/// - Traverses untracked directories to expose untracked files while collapsing fully ignored directories.
/// - Restricts traversal inside masked ignored directories to only expose tracked files and intermediate paths.
use color_eyre::eyre::Result;
use std::collections::BTreeSet;
use std::env;
use std::path::{Path, PathBuf};
use termtree::Tree;

use crate::cli::FileFilter;
use crate::commands::lib::matches_any_glob;
use crate::plan::build_plan;
use crate::status_color::status_color;
use crate::types::{Plan, Runtime};
use crate::utils::style;

#[allow(clippy::needless_pass_by_value)]
pub fn run(runtime: &Runtime, path: Option<PathBuf>, filter: Option<FileFilter>) -> Result<()> {
    let plan = build_plan(runtime, None)?;
    let root = path.unwrap_or(env::current_dir()?);
    let tracked: BTreeSet<PathBuf> = plan
        .files
        .iter()
        .map(|file| file.display_target.clone())
        .collect();

    let tracked_dirs = collect_tracked_dirs(runtime, &plan);

    let show = |status: &str| filter.as_ref().is_none_or(|f| f.as_str() == status);

    let root_display = runtime.display_path(&root);
    let root_str = format!("{}/", root_display.display());
    let root_title = format!("{} {}", style("[scan]", "36;1", runtime), root_str);

    let in_masked_tree = plan.ignored_folders.contains(&root);
    let ctx = ScanContext {
        runtime,
        tracked: &tracked,
        tracked_dirs: &tracked_dirs,
        plan: &plan,
        show: &show,
        filter: filter.as_ref(),
    };
    let tree = build_scan_tree(&ctx, &root, in_masked_tree)?;
    println!("{root_title}");
    if let Some(tree_root) = tree {
        for root_child in tree_root.leaves {
            print!("{root_child}");
        }
    }

    Ok(())
}

struct ScanContext<'a, F> {
    runtime: &'a Runtime,
    tracked: &'a BTreeSet<PathBuf>,
    tracked_dirs: &'a BTreeSet<PathBuf>,
    plan: &'a Plan,
    show: &'a F,
    filter: Option<&'a FileFilter>,
}

fn collect_tracked_dirs(runtime: &Runtime, plan: &Plan) -> BTreeSet<PathBuf> {
    let mut tracked_dirs: BTreeSet<PathBuf> = BTreeSet::new();
    for artifact in &plan.artifacts {
        for entry in walkdir::WalkDir::new(&artifact.dir) {
            let Ok(entry) = entry else { continue };
            if entry.file_type().is_dir() {
                let Ok(relative) = entry.path().strip_prefix(&artifact.dir) else {
                    continue;
                };
                if relative.as_os_str().is_empty() {
                    continue;
                }
                if let Ok((_, display_target)) = crate::plan::map_artifact_path(runtime, relative) {
                    tracked_dirs.insert(display_target);
                }
            }
        }
    }
    tracked_dirs
}

fn has_tracked_under(
    d: &Path,
    tracked: &BTreeSet<PathBuf>,
    tracked_dirs: &BTreeSet<PathBuf>,
) -> bool {
    tracked.iter().any(|file| file.starts_with(d))
        || tracked_dirs.iter().any(|dir_path| dir_path.starts_with(d))
}

fn determine_dir_status(
    path: &Path,
    leaves: &[Tree<String>],
    ctx: &ScanContext<'_, impl Fn(&str) -> bool>,
    in_masked_tree: bool,
) -> &'static str {
    let is_ignored_folder = ctx.plan.ignored_folders.contains(path);
    if is_ignored_folder {
        if has_tracked_under(path, ctx.tracked, ctx.tracked_dirs) {
            "masked"
        } else {
            "ignored"
        }
    } else if has_tracked_under(path, ctx.tracked, ctx.tracked_dirs) {
        let all_tracked = if leaves.is_empty() {
            ctx.tracked_dirs.contains(path)
        } else {
            leaves.iter().all(|leaf| leaf.root.contains("[tracked]"))
        };
        if all_tracked {
            "tracked"
        } else if in_masked_tree {
            "masked"
        } else {
            "partial"
        }
    } else if in_masked_tree || crate::ignore::is_dir_all_ignored(path, ctx.plan) {
        "ignored"
    } else {
        "untracked"
    }
}

fn format_label(status: &str, name_str: &str, runtime: &Runtime) -> String {
    let bracketed = format!("[{status}]");
    let ansi_color = status_color(status, runtime);
    format!(
        "{} {}",
        style(&format!("{bracketed:<11}"), &ansi_color, runtime),
        name_str
    )
}

fn build_scan_tree<F: Fn(&str) -> bool>(
    ctx: &ScanContext<'_, F>,
    dir: &Path,
    in_masked_tree: bool,
) -> Result<Option<Tree<String>>> {
    let mut entries = match std::fs::read_dir(dir) {
        Ok(read) => read.filter_map(Result::ok).collect::<Vec<_>>(),
        Err(_) => return Ok(None),
    };
    entries.sort_by(crate::utils::cmp_dir_entries);

    let mut children = Vec::new();
    for entry in entries {
        if let Some(node) = build_entry_node(ctx, &entry, in_masked_tree)? {
            children.push(node);
        }
    }

    let root_node_name = dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let mut root_tree = Tree::new(root_node_name);
    for child in children {
        root_tree.push(child);
    }
    Ok(Some(root_tree))
}

fn build_entry_node<F: Fn(&str) -> bool>(
    ctx: &ScanContext<'_, F>,
    entry: &std::fs::DirEntry,
    in_masked_tree: bool,
) -> Result<Option<Tree<String>>> {
    let path = entry.path();
    let is_dir = path.is_dir();
    let name = entry.file_name().to_string_lossy().to_string();
    let name_str = if is_dir { format!("{name}/") } else { name };

    let show_ignored = ctx.filter.is_some_and(|f| f.as_str() == "ignored");

    if !is_dir {
        let status = if in_masked_tree {
            if ctx.tracked.contains(&path) {
                "tracked"
            } else {
                "ignored"
            }
        } else if matches_any_glob(&path, &ctx.plan.ignored_files) {
            "ignored"
        } else if ctx.tracked.contains(&path) {
            "tracked"
        } else {
            "untracked"
        };

        let should_show = if in_masked_tree && status == "ignored" {
            show_ignored
        } else {
            (ctx.show)(status)
        };

        if should_show {
            return Ok(Some(Tree::new(format_label(
                status,
                &name_str,
                ctx.runtime,
            ))));
        }
        return Ok(None);
    }

    let is_ignored_folder = ctx.plan.ignored_folders.contains(&path);
    let next_in_masked = in_masked_tree || is_ignored_folder;

    let initial_status = if is_ignored_folder {
        if has_tracked_under(&path, ctx.tracked, ctx.tracked_dirs) {
            "masked"
        } else {
            "ignored"
        }
    } else if has_tracked_under(&path, ctx.tracked, ctx.tracked_dirs) {
        if in_masked_tree { "masked" } else { "partial" }
    } else if in_masked_tree || crate::ignore::is_dir_all_ignored(&path, ctx.plan) {
        "ignored"
    } else {
        "untracked"
    };

    if initial_status == "ignored" {
        let should_show = if in_masked_tree {
            show_ignored
        } else {
            (ctx.show)("ignored")
        };
        if should_show {
            return Ok(Some(Tree::new(format_label(
                "ignored",
                &name_str,
                ctx.runtime,
            ))));
        }
        return Ok(None);
    }

    let sub_tree = build_scan_tree(ctx, &path, next_in_masked)?;
    let leaves = sub_tree.map_or_else(Vec::new, |sub| sub.leaves);

    let dir_status = determine_dir_status(&path, &leaves, ctx, in_masked_tree);

    if dir_status == "ignored" {
        let should_show = if in_masked_tree {
            show_ignored
        } else {
            (ctx.show)("ignored")
        };
        if should_show {
            return Ok(Some(Tree::new(format_label(
                "ignored",
                &name_str,
                ctx.runtime,
            ))));
        }
        return Ok(None);
    }

    let mut node = Tree::new(format_label(dir_status, &name_str, ctx.runtime));
    for child in leaves {
        node.push(child);
    }

    let should_show_node = if in_masked_tree && dir_status == "masked" {
        !node.leaves.is_empty() || show_ignored
    } else {
        (ctx.show)(dir_status) || !node.leaves.is_empty()
    };

    if should_show_node {
        Ok(Some(node))
    } else {
        Ok(None)
    }
}
