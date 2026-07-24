/// CLI Command: `deploy apply [artifact] [-y]`
///
/// What it does:
/// Applies planned file and generated-environment changes, and prints copyable commands for external installs.
///
/// Variations:
/// 1. `artifact` provided: Applies only changes and dependencies associated with that specific artifact.
/// 2. `-y` / `--yes` provided: Skips the file-write confirmation prompt.
/// 3. `-y` not provided: Asks once before pending file writes.
///
/// Decisions & Logic Branches:
/// - Builds the deployment plan and checks for changes:
///   - Compares target file bytes against planned source bytes.
///   - Verifies missing native packages, Flatpaks, and downloads for command output.
/// - Exits early with "No changes to apply" if the system is fully up to date.
/// - Never invokes package managers, Flatpak, curl, unzip, or other external installers.
/// - Copies/deploys pending files to their targets.
/// - Prints shell-ready commands for missing packages, Flatpaks, and downloads.
/// - Writes generated environment variables to the configured environment path.
use color_eyre::eyre::Result;
use std::fs;

use crate::commands::lib::{
    apply_file, apply_packages_and_downloads, print_plan_extras, write_env_file,
};
use crate::plan::build_plan;
use crate::types::{DownloadInstall, Runtime};
use crate::utils::{confirm, style};

fn check_artifact_has_changes(art: &crate::types::Artifact, plan: &crate::types::Plan) -> bool {
    for file in plan.files.iter().filter(|f| f.artifact_id == art.id) {
        if file.target.exists() {
            let current = fs::read(&file.target).unwrap_or_default();
            if current != file.bytes {
                return true;
            }
        } else {
            return true;
        }
    }
    false
}

fn print_artifact_changes(
    runtime: &Runtime,
    art: &crate::types::Artifact,
    plan: &crate::types::Plan,
) {
    println!("Changes for {}:", art.id);
    for file in plan.files.iter().filter(|file| file.artifact_id == art.id) {
        if file.target.exists() {
            let current = fs::read(&file.target).unwrap_or_default();
            if current != file.bytes {
                println!(
                    "  {} {} -> {}",
                    style("[change]", "33", runtime),
                    runtime.display_path(&file.source).display(),
                    runtime.display_path(&file.display_target).display()
                );
            }
        } else {
            println!(
                "  {} {} -> {}",
                style("[new]", "32", runtime),
                runtime.display_path(&file.source).display(),
                runtime.display_path(&file.display_target).display()
            );
        }
    }

    let mut missing_pkgs = Vec::new();
    for (distro, pkg_set) in &art.bin.distro {
        for pkg in &pkg_set.packages {
            if !crate::utils::is_package_installed(distro, pkg) {
                missing_pkgs.push(format!("native ({distro}): {pkg}"));
            }
        }
    }
    for flatpak in &art.bin.flatpak.packages {
        if !crate::utils::is_flatpak_installed(flatpak) {
            missing_pkgs.push(format!("flatpak: {flatpak}"));
        }
    }
    for download in plan.downloads.iter().filter(|d| d.artifact_id == art.id) {
        if !download.install_path.exists() {
            let install = match download.install {
                DownloadInstall::Local => "local",
                DownloadInstall::System => "system",
            };
            missing_pkgs.push(format!(
                "download ({install}): {}",
                runtime.display_path(&download.display_path).display()
            ));
        }
    }

    if !missing_pkgs.is_empty() {
        println!("  Packages/Downloads to install:");
        for pkg in missing_pkgs {
            println!("    - {pkg}");
        }
    }
}

pub fn run(runtime: &Runtime, args: &crate::cli::ApplyArgs) -> Result<()> {
    crate::utils::print_banner("APPLYING ARTIFACTS", runtime);
    let plan = build_plan(runtime, args.artifact.as_deref())?;

    let mut artifacts_to_apply = Vec::new();
    for art in &plan.artifacts {
        if check_artifact_has_changes(art, &plan) {
            artifacts_to_apply.push(art.clone());
        }
    }

    if artifacts_to_apply.is_empty() {
        let external = apply_packages_and_downloads(runtime, &plan, args.yes)?;
        if !external {
            println!("No changes to apply.");
        }
        return Ok(());
    }

    for art in artifacts_to_apply {
        if !args.yes {
            print_artifact_changes(runtime, &art, &plan);
        }

        if !args.yes && !confirm(&format!("apply {}?", art.id), runtime.no_color)? {
            continue;
        }

        for file in plan.files.iter().filter(|file| file.artifact_id == art.id) {
            apply_file(runtime, file, args.yes)?;
        }
    }
    apply_packages_and_downloads(runtime, &plan, args.yes)?;
    write_env_file(runtime, &plan)?;
    print_plan_extras(runtime, &plan, None);
    Ok(())
}
