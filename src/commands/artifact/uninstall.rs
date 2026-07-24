/// CLI Command: `artifact uninstall <artifact_id> [-y]`
///
/// What it does:
/// Removes safely matching deployed files, reports exclusive dependency claims, and disables an artifact.
///
/// Variations:
/// 1. `-y` / `--yes` provided: Skips interactive confirmation.
/// 2. `-y` not provided: Prompts the user before proceeding with deletion.
///
/// Decisions & Logic Branches:
/// - Fails if the specified `artifact_id` is not found or has no files.
/// - Identifies which target files/paths exist and determines if they can be safely removed:
///   - Deployed files are only removed if their content matches the original source bytes (otherwise they are skipped to prevent overwriting user edits).
///   - Additional files/directories listed under the artifact's `bin.config.remove` block are removed recursively.
/// - Backs up all removed files/folders before deletion.
/// - Analyzes which installed dependencies (packages, Flatpaks, or downloads) are no longer claimed by another enabled artifact and prints cleanup hints; it never removes them automatically.
/// - Automatically calls `artifact disable` at the end to disable the artifact in settings.
use color_eyre::eyre::{Result, bail};
use std::fs;
use std::path::PathBuf;

use crate::commands::lib::{collect_claims, print_unclaimed_hints};
use crate::plan::build_plan;
use crate::types::{Artifact, Plan, Runtime};
use crate::utils::{backup_file, confirm};

fn preview_removal(runtime: &Runtime, target_plan: &Plan) -> Result<()> {
    println!("The following files and paths will be removed:");
    for file in &target_plan.files {
        if file.target.exists() {
            let current = fs::read(&file.target)?;
            if current == file.bytes {
                println!(
                    "  - file: {}",
                    runtime.display_path(&file.display_target).display()
                );
            } else {
                println!(
                    "  - file (will skip - content changed): {}",
                    runtime.display_path(&file.display_target).display()
                );
            }
        }
    }
    for path_str in &target_plan.artifacts[0].bin.config.remove {
        let path = runtime.resolve_tilde(path_str);
        let display_path = if let Ok(rest) = path.strip_prefix(&runtime.home_dir) {
            PathBuf::from("~").join(rest)
        } else {
            path.clone()
        };
        if path.exists() {
            if path.is_dir() {
                println!("  - directory (remove): {}", display_path.display());
            } else {
                println!("  - file (remove): {}", display_path.display());
            }
        }
    }
    println!();
    Ok(())
}

fn perform_removal(runtime: &Runtime, target_plan: &Plan) -> Result<()> {
    for file in &target_plan.files {
        if !file.target.exists() {
            println!(
                "missing {}",
                runtime.display_path(&file.display_target).display()
            );
            continue;
        }
        let current = fs::read(&file.target)?;
        if current == file.bytes {
            backup_file(runtime, &file.target, &file.display_target)?;
            fs::remove_file(&file.target)?;
            println!(
                "removed {}",
                runtime.display_path(&file.display_target).display()
            );
        } else {
            println!(
                "skip changed {}",
                runtime.display_path(&file.display_target).display()
            );
        }
    }

    for path_str in &target_plan.artifacts[0].bin.config.remove {
        let path = runtime.resolve_tilde(path_str);
        let display_path = if let Ok(rest) = path.strip_prefix(&runtime.home_dir) {
            PathBuf::from("~").join(rest)
        } else {
            path.clone()
        };
        if path.exists() {
            backup_file(runtime, &path, &display_path)?;
            if path.is_dir() {
                fs::remove_dir_all(&path)?;
            } else {
                fs::remove_file(&path)?;
            }
            println!("Removed {}", display_path.display());
        }
    }
    Ok(())
}

pub fn run(runtime: &Runtime, artifact_id: &str, yes: bool) -> Result<()> {
    crate::utils::print_banner("REMOVING ARTIFACT", runtime);
    let target_plan = build_plan(runtime, Some(artifact_id))?;
    if target_plan.artifacts.is_empty() {
        bail!("artifact not found: {artifact_id}");
    }

    preview_removal(runtime, &target_plan)?;

    if !yes
        && !confirm(
            &format!("remove deployed files for {artifact_id}?"),
            runtime.no_color,
        )?
    {
        return Ok(());
    }

    perform_removal(runtime, &target_plan)?;

    let current_plan = build_plan(runtime, None)?;
    let other_artifacts: Vec<Artifact> = current_plan
        .artifacts
        .iter()
        .filter(|artifact| artifact.id != artifact_id)
        .cloned()
        .collect();
    let target_claims = collect_claims(runtime, &target_plan.artifacts)?;
    let other_claims = collect_claims(runtime, &other_artifacts)?;
    print_unclaimed_hints(runtime, &target_claims, &other_claims);

    crate::commands::artifact::disable::run(runtime, artifact_id)?;
    Ok(())
}
