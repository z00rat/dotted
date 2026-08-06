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

use crate::commands::deploy::files::{self, FileChange};
use crate::commands::lib::print_plan_extras;
use crate::plan::build_plan;
use crate::types::Runtime;
use crate::utils::style;

fn print_artifact_files_status(
    runtime: &Runtime,
    plan: &crate::types::Plan,
    art_id: &str,
    indent: &str,
) -> Result<()> {
    let art_files: Vec<_> = plan
        .files
        .iter()
        .filter(|f| f.artifact_id == art_id)
        .collect();
    for file in art_files {
        match files::classify(file)? {
            Some(FileChange::Changed) => println!(
                "{indent}{} {} -> {}",
                style("[change]", "33", runtime),
                runtime.display_path(&file.source).display(),
                runtime.display_path(&file.display_target).display()
            ),
            Some(FileChange::New) => println!(
                "{indent}{} {} -> {}",
                style("[new]", "32", runtime),
                runtime.display_path(&file.source).display(),
                runtime.display_path(&file.display_target).display()
            ),
            None => {}
        }
    }
    Ok(())
}

fn print_artifact_environment(artifact: &crate::types::Artifact) {
    if artifact.bin.env.is_empty() {
        return;
    }
    println!("    env:");
    for (key, value) in &artifact.bin.env {
        println!("      {key} = \"{value}\"");
    }
}

fn print_artifact_packages(runtime: &Runtime, artifact: &crate::types::Artifact) {
    if artifact.bin.distro.is_empty() && artifact.bin.flatpak.packages.is_empty() {
        return;
    }
    println!("    packages:");
    for (distro, package_set) in &artifact.bin.distro {
        for package in &package_set.packages {
            let tag = if crate::utils::is_package_installed(distro, package) {
                style("[installed]", "34", runtime)
            } else {
                style("[missing]", "32", runtime)
            };
            println!("      {tag} native ({distro}): {package}");
        }
    }
    for flatpak in &artifact.bin.flatpak.packages {
        let tag = if crate::utils::is_flatpak_installed(flatpak) {
            style("[installed]", "34", runtime)
        } else {
            style("[missing]", "32", runtime)
        };
        println!("      {tag} flatpak: {flatpak}");
    }
}

fn print_artifact_services(runtime: &Runtime, artifact: &crate::types::Artifact) {
    if artifact.bin.services.is_empty() {
        return;
    }
    println!("    services:");
    for (scope, service_set) in &artifact.bin.services {
        if service_set.units.is_empty() {
            continue;
        }
        println!("      {scope}:");
        for unit in &service_set.units {
            let enabled = crate::utils::is_service_enabled(scope, unit);
            let active = crate::utils::is_service_active(scope, unit);
            let status = match (enabled, active) {
                (true, true) => style("[active & enabled]", "34", runtime),
                (true, false) => style("[inactive & enabled]", "33", runtime),
                (false, true) => style("[active & disabled]", "33", runtime),
                (false, false) => style("[disabled]", "32", runtime),
            };
            println!("        {status} {unit}");
        }
    }
}

fn print_artifact_downloads(runtime: &Runtime, plan: &crate::types::Plan, artifact_id: &str) {
    let downloads: Vec<_> = plan
        .downloads
        .iter()
        .filter(|download| download.artifact_id == artifact_id)
        .collect();
    if downloads.is_empty() {
        return;
    }
    println!("    downloads:");
    for download in downloads {
        let tag = if download.install_path.exists() {
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

fn print_artifacts_section(
    runtime: &Runtime,
    plan: &crate::types::Plan,
    show: impl Fn(&str) -> bool,
) -> Result<()> {
    println!("artifacts:");
    for artifact in &plan.artifacts {
        println!(
            "  {} r{} ({}) - {}",
            style(&artifact.id, "36;1", runtime),
            artifact.revision,
            artifact.name,
            artifact.description
        );
        if show("files") && files::has_changes(plan, &artifact.id)? {
            println!("    files:");
            print_artifact_files_status(runtime, plan, &artifact.id, "      ")?;
        }
        if show("env") {
            print_artifact_environment(artifact);
        }
        if show("packages") {
            print_artifact_packages(runtime, artifact);
        }
        if show("services") {
            print_artifact_services(runtime, artifact);
        }
        if show("downloads") {
            print_artifact_downloads(runtime, plan, &artifact.id);
        }
    }
    Ok(())
}

pub fn run(runtime: &Runtime, artifact: Option<&str>, filter: Option<&str>) -> Result<()> {
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
        print_artifacts_section(runtime, &plan, show)?;
    } else {
        if show("files") {
            println!("files:");
            for file in &plan.files {
                match files::classify(file)? {
                    Some(FileChange::Changed) => println!(
                        "  {} {} {} -> {}",
                        style("[change]", "33", runtime),
                        file.artifact_id,
                        runtime.display_path(&file.source).display(),
                        runtime.display_path(&file.display_target).display()
                    ),
                    Some(FileChange::New) => println!(
                        "  {} {} {} -> {}",
                        style("[new]", "32", runtime),
                        file.artifact_id,
                        runtime.display_path(&file.source).display(),
                        runtime.display_path(&file.display_target).display()
                    ),
                    None => {}
                }
            }
        }
        print_plan_extras(runtime, &plan, filter);
    }
    Ok(())
}
