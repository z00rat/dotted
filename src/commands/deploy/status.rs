/// CLI Command: `deploy status [artifact] [--filter <artifacts|files|env|packages|downloads>]`
///
/// What it does:
/// Displays host information, active artifacts, pending file changes (new or modified files), and required package installations.
///
/// Variations:
/// 1. `artifact` filter provided: Restricts status checks to files and packages from that specific artifact.
/// 2. `--filter` provided: Limits output to the selected status category.
///
/// Decisions & Logic Branches:
/// - Builds the deployment plan based on current settings and active artifacts.
/// - Iterates over all planned files:
///   - Skips files that are already deployed and have identical contents.
///   - Marks existing files with differing contents as `[change]`.
///   - Marks non-existent files as `[new]`.
/// - Prints the status of environment variables, packages, flatpaks, and downloads matching the filter.
use color_eyre::eyre::Result;
use std::fs;

use crate::commands::lib::print_plan_extras;
use crate::plan::build_plan;
use crate::types::Runtime;
use crate::utils::style;

fn print_artifact_files_status(
    runtime: &Runtime,
    plan: &crate::types::Plan,
    art_id: &str,
    indent: &str,
) {
    let art_files: Vec<_> = plan
        .files
        .iter()
        .filter(|f| f.artifact_id == art_id)
        .collect();
    for file in art_files {
        if file.target.exists() {
            let current = fs::read(&file.target).unwrap_or_default();
            if current == file.bytes {
                continue;
            }
            println!(
                "{indent}{} {} -> {}",
                style("[change]", "33", runtime),
                runtime.display_path(&file.source).display(),
                runtime.display_path(&file.display_target).display()
            );
        } else {
            println!(
                "{indent}{} {} -> {}",
                style("[new]", "32", runtime),
                runtime.display_path(&file.source).display(),
                runtime.display_path(&file.display_target).display()
            );
        }
    }
}

fn print_artifacts_section(
    runtime: &Runtime,
    plan: &crate::types::Plan,
    show: impl Fn(&str) -> bool,
) {
    println!("artifacts:");
    for art in &plan.artifacts {
        println!(
            "  {} r{} ({}) - {}",
            style(&art.id, "36;1", runtime),
            art.revision,
            art.name,
            art.description
        );

        if show("files") {
            let has_file_changes = plan
                .files
                .iter()
                .filter(|f| f.artifact_id == art.id)
                .any(|f| !f.target.exists() || fs::read(&f.target).unwrap_or_default() != f.bytes);
            if has_file_changes {
                println!("    files:");
                print_artifact_files_status(runtime, plan, &art.id, "      ");
            }
        }

        if show("env") && !art.bin.env.is_empty() {
            println!("    env:");
            for (k, v) in &art.bin.env {
                println!("      {k} = \"{v}\"");
            }
        }

        if show("packages") && (!art.bin.distro.is_empty() || !art.bin.flatpak.packages.is_empty())
        {
            println!("    packages:");
            for (distro, pkg_set) in &art.bin.distro {
                for pkg in &pkg_set.packages {
                    let installed = crate::utils::is_package_installed(distro, pkg);
                    let tag = if installed {
                        style("[installed]", "34", runtime)
                    } else {
                        style("[missing]", "32", runtime)
                    };
                    println!("      {tag} native ({distro}): {pkg}");
                }
            }
            for flatpak in &art.bin.flatpak.packages {
                let installed = crate::utils::is_flatpak_installed(flatpak);
                let tag = if installed {
                    style("[installed]", "34", runtime)
                } else {
                    style("[missing]", "32", runtime)
                };
                println!("      {tag} flatpak: {flatpak}");
            }
        }

        if show("downloads") && plan.downloads.iter().any(|d| d.artifact_id == art.id) {
            println!("    downloads:");
            for download in plan.downloads.iter().filter(|d| d.artifact_id == art.id) {
                let installed = download.install_path.exists();
                let tag = if installed {
                    style("[installed]", "34", runtime)
                } else {
                    style("[missing]", "32", runtime)
                };
                println!(
                    "      {tag} {} -> {}",
                    download.url_or_zip_url(),
                    download.display_path.display()
                );
            }
        }
    }
}

pub fn run(runtime: &Runtime, artifact: Option<&str>, filter: Option<&str>) -> Result<()> {
    crate::utils::print_banner("STATUS REPORT", runtime);
    let plan = build_plan(runtime, artifact)?;

    let show = |section: &str| filter.is_none_or(|f| f == section);

    println!("device: {}", runtime.device);
    println!("user: {}", runtime.user);
    println!(
        "dotted: {}",
        runtime.display_path(&runtime.dotted_dir).display()
    );
    println!();

    if show("artifacts") {
        print_artifacts_section(runtime, &plan, show);
    } else {
        if show("files") {
            println!("files:");
            for file in &plan.files {
                if file.target.exists() {
                    let current = fs::read(&file.target).unwrap_or_default();
                    if current == file.bytes {
                        continue;
                    }
                    println!(
                        "  {} {} {} -> {}",
                        style("[change]", "33", runtime),
                        file.artifact_id,
                        runtime.display_path(&file.source).display(),
                        runtime.display_path(&file.display_target).display()
                    );
                } else {
                    println!(
                        "  {} {} {} -> {}",
                        style("[new]", "32", runtime),
                        file.artifact_id,
                        runtime.display_path(&file.source).display(),
                        runtime.display_path(&file.display_target).display()
                    );
                }
            }
        }
        print_plan_extras(runtime, &plan, filter);
    }
    Ok(())
}
