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
/// - Traverses untracked directories to expose untracked files while collapsing fully ignored directories.
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

    let show = |status: &str| filter.as_ref().is_none_or(|f| f.as_str() == status);

    let root_display = runtime.display_path(&root);
    let root_str = format!("{}/", root_display.display());
    let root_title = format!("{} {}", style("[scan]", "36;1", runtime), root_str);

    let tree = build_scan_tree(runtime, &root, &tracked, &plan, &show)?;
    println!("{root_title}");
    if let Some(tree_root) = tree {
        for root_child in tree_root.leaves {
            print!("{root_child}");
        }
    }

    Ok(())
}

fn build_scan_tree(
    runtime: &Runtime,
    dir: &Path,
    tracked: &BTreeSet<PathBuf>,
    plan: &Plan,
    show: &impl Fn(&str) -> bool,
) -> Result<Option<Tree<String>>> {
    let mut entries = match std::fs::read_dir(dir) {
        Ok(read) => read.filter_map(Result::ok).collect::<Vec<_>>(),
        Err(_) => return Ok(None),
    };
    entries.sort_by_key(std::fs::DirEntry::path);

    let mut children = Vec::new();
    let has_tracked_files_under = |d: &Path| tracked.iter().any(|file| file.starts_with(d));

    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type()?;
        let is_dir = file_type.is_dir();
        let is_ignored_folder = is_dir && plan.ignored_folders.contains(&path);
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
            if has_tracked_files_under(&path) {
                "partial"
            } else {
                "untracked"
            }
        } else if tracked.contains(&path) {
            "tracked"
        } else {
            "untracked"
        };

        let name = entry.file_name().to_string_lossy().to_string();
        let name_str = if is_dir { format!("{name}/") } else { name };
        let bracketed = format!("[{status}]");
        let ansi_color = status_color(status, runtime);
        let styled_label = format!(
            "{} {}",
            style(&format!("{bracketed:<11}"), &ansi_color, runtime),
            name_str
        );

        if is_dir {
            if is_ignored_folder && status == "ignored" {
                if show(status) {
                    children.push(Tree::new(styled_label));
                }
            } else {
                let sub_tree = build_scan_tree(runtime, &path, tracked, plan, show)?;
                let mut leaves = Vec::new();
                if let Some(sub) = sub_tree {
                    leaves = sub.leaves;
                }

                // Recalculate directory status based on actual contents
                let dir_status = if is_ignored_folder {
                    if has_tracked_files_under(&path) {
                        "masked"
                    } else {
                        "ignored"
                    }
                } else if has_tracked_files_under(&path) {
                    let all_tracked = !leaves.is_empty()
                        && leaves.iter().all(|leaf| leaf.root.contains("[tracked]"));
                    if all_tracked { "tracked" } else { "partial" }
                } else {
                    "untracked"
                };

                let bracketed = format!("[{dir_status}]");
                let ansi_color = status_color(dir_status, runtime);
                let final_styled_label = format!(
                    "{} {}",
                    style(&format!("{bracketed:<11}"), &ansi_color, runtime),
                    name_str
                );

                let mut node = Tree::new(final_styled_label);
                for child in leaves {
                    node.push(child);
                }

                if show(dir_status) || !node.leaves.is_empty() {
                    children.push(node);
                }
            }
        } else if show(status) {
            children.push(Tree::new(styled_label));
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
