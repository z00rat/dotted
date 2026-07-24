/// CLI Command: `adopt package <artifact_id> [package] --type <type>`
///
/// Records a native-distro or Flatpak package in an artifact without enabling it.
/// The package type selector is interactive when omitted, and non-interactive runs
/// require both the package name and `--type`.
use color_eyre::eyre::{Result, bail};

use crate::commands::lib::{ensure_about_entry, repository_path, split_artifact_id};
use crate::types::{BIN_TOML, BinFile, Runtime};
use crate::utils::style;

fn prompt_package_name(runtime: &Runtime, package: Option<String>) -> Result<String> {
    if let Some(p) = package {
        Ok(p)
    } else if !runtime.no_color {
        cliclack::input("Enter package name to adopt:")
            .placeholder("git")
            .interact::<String>()
            .map(|s| s.trim().to_string())
            .map_err(|e| color_eyre::eyre::Report::msg(e.to_string()))
    } else {
        bail!("adopt package requires a package name when running non-interactively");
    }
}

fn prompt_type(runtime: &Runtime, package_type: Option<String>) -> Result<String> {
    if let Some(m) = package_type {
        Ok(m.trim().to_ascii_lowercase())
    } else if !runtime.no_color {
        let choice = cliclack::select("Type")
            .item(
                runtime.distro.clone(),
                format!("Native ({})", runtime.distro),
                "",
            )
            .item("flatpak".to_string(), "Flatpak", "")
            .interact()
            .map_err(|e| color_eyre::eyre::Report::msg(e.to_string()))?;
        Ok(choice.clone())
    } else {
        Ok(runtime.distro.clone())
    }
}

pub fn run(
    runtime: &Runtime,
    artifact_id: &str,
    package: Option<String>,
    package_type: Option<String>,
) -> Result<()> {
    let (repo, artifact) = split_artifact_id(artifact_id)?;

    let artifact_dir = repository_path(runtime, repo).join(artifact);
    if !artifact_dir.exists() {
        bail!(
            "artifact directory does not exist: {}",
            artifact_dir.display()
        );
    }

    let package_name = prompt_package_name(runtime, package)?;
    let package_type = prompt_type(runtime, package_type)?;

    let bin_path = artifact_dir.join(BIN_TOML);
    let mut bin_file: BinFile = if bin_path.exists() {
        crate::types::read_toml(&bin_path)?
    } else {
        BinFile::default()
    };

    if package_type == "flatpak" {
        if !bin_file.flatpak.packages.contains(&package_name) {
            bin_file.flatpak.packages.push(package_name.clone());
        }
    } else {
        let set = bin_file.distro.entry(package_type.clone()).or_default();
        if !set.packages.contains(&package_name) {
            set.packages.push(package_name.clone());
        }
    }

    crate::types::write_toml(&bin_path, &bin_file)?;
    ensure_about_entry(runtime, repo, artifact)?;
    if package_type == "flatpak" {
        println!(
            "Added flatpak package {} to {} [bin].toml",
            style(&package_name, "32", runtime),
            style(artifact_id, "36;1", runtime)
        );
    } else {
        println!(
            "Added native ({}) package {} to {} [bin].toml",
            package_type,
            style(&package_name, "32", runtime),
            style(artifact_id, "36;1", runtime)
        );
    }

    Ok(())
}
