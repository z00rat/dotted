/// CLI Command: `deploy diff [artifact]`
///
/// What it does:
/// Displays line-by-line differences between the planned dotfiles/templates and the current state on the filesystem.
///
/// Variations:
/// 1. `artifact` filter provided: Shows diffs only for files from that specific artifact.
///
/// Decisions & Logic Branches:
/// - Builds the deployment plan based on current settings and active artifacts.
/// - Identifies two groups of files to display:
///   - Missing files (files that do not exist at the target path).
///   - Changed files (files that exist but have differing content bytes).
/// - Displays the list of missing files first.
/// - Renders colorful side-by-side/inline diff blocks for all changed files.
use color_eyre::eyre::Result;
use std::fs;

use crate::plan::build_plan;
use crate::types::Runtime;
use crate::utils::{show_file_diff, style};

pub fn run(runtime: &Runtime, artifact: Option<&str>, filter: Option<&str>) -> Result<()> {
    crate::utils::print_banner("DIFFERENCES DETECTED", runtime);
    let plan = build_plan(runtime, artifact)?;
    let matches_filter = |s: &str| -> bool {
        if let Some(f) = filter {
            s.to_lowercase().contains(&f.to_lowercase())
        } else {
            true
        }
    };

    let mut missing_files = Vec::new();
    let mut changed_files = Vec::new();

    for file in &plan.files {
        let path_str = file.display_target.to_string_lossy();
        if !matches_filter(&file.artifact_id) && !matches_filter(&path_str) {
            continue;
        }
        if file.target.exists() {
            let current = fs::read(&file.target)?;
            if current != file.bytes {
                changed_files.push((file, current));
            }
        } else {
            missing_files.push(file);
        }
    }

    if !missing_files.is_empty() {
        println!("{}", style("Missing Files:", "31;1", runtime));
        for file in &missing_files {
            println!(
                "  - {} ({})",
                runtime.display_path(&file.display_target).display(),
                file.artifact_id
            );
        }
        println!();
    }

    for (file, current) in changed_files {
        let border = "━".repeat(80);
        println!("{}", style(&border, "33", runtime));
        println!(
            "{}  {} ({})",
            style("[diff]", "33;1", runtime),
            style(
                &runtime.display_path(&file.display_target).to_string_lossy(),
                "1",
                runtime
            ),
            file.artifact_id
        );
        println!("{}", style(&border, "33", runtime));
        show_file_diff(file, &current, runtime);
        println!("{}", style(&border, "33", runtime));
        println!();
    }

    Ok(())
}
