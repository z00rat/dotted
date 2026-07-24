/// CLI Command: `artifact show <artifact_id>`
///
/// What it does:
/// Displays detailed configuration, files, deployment status, and file diffs for a specific artifact.
///
/// Variations:
/// None (requires an exact `artifact_id` argument).
///
/// Decisions & Logic Branches:
/// - Fails if the requested `artifact_id` does not exist in the workspace.
/// - Performs a `WalkDir` over the artifact's source folder to list all tracked files (ignoring `[bin].toml`).
/// - Builds a deployment plan restricted to this artifact, and checks each target file's status:
///   - `[ok]`: Target file exists and matches the source content.
///   - `[changed]`: Target file exists but has modifications.
///   - `[missing]`: Target file does not exist on the filesystem.
/// - Shows inline git-like diffs for any `[changed]` files.
use color_eyre::eyre::{Result, bail};
use std::fs;
use walkdir::WalkDir;

use crate::commands::lib::print_plan_extras;
use crate::plan::{build_plan, discover_artifacts};
use crate::types::Runtime;
use crate::utils::{show_file_diff, style};

fn print_artifact_metadata(runtime: &Runtime, artifact: &crate::types::Artifact) {
    println!("ID:          {}", style(&artifact.id, "36;1", runtime));
    println!("Revision:    r{}", artifact.revision);
    println!(
        "Directory:   {}",
        runtime.display_path(&artifact.dir).display()
    );
    println!("Description: {}", artifact.description);
    println!();
}

fn print_binaries_and_deps(runtime: &Runtime, artifact: &crate::types::Artifact) {
    println!("{}", style("Dependencies:", "36;1", runtime));
    if !artifact.bin.env.is_empty() {
        println!("Environment Variables:");
        for (k, v) in &artifact.bin.env {
            println!("  {k} = \"{v}\"");
        }
    }
    if !artifact.bin.distro.is_empty() {
        println!("Native Packages:");
        for (distro, pkg_set) in &artifact.bin.distro {
            println!("  [{distro}]: {}", pkg_set.packages.join(", "));
        }
    }
    if !artifact.bin.flatpak.packages.is_empty() {
        println!("Flatpak Packages:");
        println!("  {}", artifact.bin.flatpak.packages.join(", "));
    }
    if !artifact.bin.download.is_empty() {
        println!("Downloads:");
        for (arch, dl_spec) in &artifact.bin.download {
            let url = dl_spec
                .url
                .as_deref()
                .unwrap_or(dl_spec.zip.as_deref().unwrap_or(""));
            let path = dl_spec.path.as_deref().unwrap_or("");
            println!("  [{arch}]: {url} -> {path}");
        }
    }
    println!();
}

fn print_artifact_files(_runtime: &Runtime, artifact: &crate::types::Artifact) -> Result<()> {
    println!("Files:");
    let mut walked_any = false;
    for entry in WalkDir::new(&artifact.dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let rel_path = entry.path().strip_prefix(&artifact.dir)?;
            let rel_str = rel_path.to_string_lossy();
            if rel_str == "[bin].toml" {
                continue;
            }
            println!("  - {rel_str}");
            walked_any = true;
        }
    }
    if !walked_any {
        println!("  No files under this artifact.");
    }
    println!();
    Ok(())
}

fn print_deployment_status(runtime: &Runtime, plan: &crate::types::Plan) {
    println!("Deployment status:");
    for file in &plan.files {
        if file.target.exists() {
            let current = fs::read(&file.target).unwrap_or_default();
            if current == file.bytes {
                println!(
                    "  {} {}",
                    style("[ok]", "32", runtime),
                    runtime.display_path(&file.display_target).display()
                );
            } else {
                println!(
                    "  {} {}",
                    style("[changed]", "33", runtime),
                    runtime.display_path(&file.display_target).display()
                );
            }
        } else {
            println!(
                "  {} {}",
                style("[missing]", "31", runtime),
                runtime.display_path(&file.display_target).display()
            );
        }
    }
    print_plan_extras(runtime, plan, None);
    println!();
}

pub fn run(runtime: &Runtime, artifact_id: &str) -> Result<()> {
    let artifacts = discover_artifacts(runtime)?;
    let Some(artifact) = artifacts.get(artifact_id) else {
        bail!("Artifact '{artifact_id}' not found in the workspace.");
    };

    print_artifact_metadata(runtime, artifact);
    print_binaries_and_deps(runtime, artifact);
    print_artifact_files(runtime, artifact)?;

    // Deployment status
    let plan = build_plan(runtime, Some(artifact_id))?;
    print_deployment_status(runtime, &plan);

    // Diffs
    println!("{}", style("DEPLOYMENT DIFFS:", "36;1", runtime));
    let mut diff_shown = false;
    for file in &plan.files {
        if file.target.exists() {
            let current = fs::read(&file.target)?;
            if current != file.bytes {
                let border = "━".repeat(80);
                println!("{}", style(&border, "33", runtime));
                println!(
                    "{}  {} ({})",
                    style("[diff]", "33;1", runtime),
                    style(&file.display_target.to_string_lossy(), "1", runtime),
                    file.artifact_id
                );
                println!("{}", style(&border, "33", runtime));
                show_file_diff(file, &current, runtime);
                println!("{}", style(&border, "33", runtime));
                println!();
                diff_shown = true;
            }
        }
    }
    if !diff_shown {
        println!("  No file diffs detected.");
    }

    Ok(())
}
