/// CLI Command: `deploy orphans [--filter <native|flatpak|downloads>]`
///
/// What it does:
/// Generates an inventory report comparing installed system resources (native packages, flatpaks, and binaries in bin folders) against what is currently declared and managed in active artifacts.
///
/// Variations:
/// 1. `--filter` provided: Displays only the selected resource category.
///
/// Decisions & Logic Branches:
/// - Builds the deployment plan to understand all declared assets.
/// - Compares the set of packages currently installed on the host OS with the set of declared native packages. Shows "unclaimed" packages.
/// - Compares the set of flatpaks installed on the host OS with the set of declared flatpaks. Shows "unclaimed" flatpaks.
/// - Scans target binary directories (`~/.local/bin` and `/usr/local/bin`) to find files that do not correspond to any declared download or deployed file, highlighting them as "unclaimed".
/// - Acts strictly as an informational audit tool: it never modifies or deletes any files or packages on its own.
use color_eyre::eyre::Result;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::OutputFormat;
use crate::commands::lib::{installed_flatpaks, installed_native_packages};
use crate::plan::build_plan;
use crate::types::Runtime;
use crate::utils::style;

fn print_native_packages_inventory(
    runtime: &Runtime,
    plan: &crate::types::Plan,
    filter: Option<&str>,
) {
    if filter.is_some_and(|f| f != "native") {
        return;
    }
    println!("{}", style("Native packages", "36;1", runtime));
    for (distro, packages) in &plan.packages {
        let pkgs_list = if packages.is_empty() {
            "(none)".to_string()
        } else {
            packages.iter().cloned().collect::<Vec<_>>().join(" ")
        };
        println!("  Installed/Declared: {pkgs_list}");
        match installed_native_packages(distro) {
            Ok(installed) => {
                let extra: Vec<_> = installed
                    .difference(packages)
                    .filter(|pkg| !plan.ignored_packages.contains(*pkg))
                    .cloned()
                    .collect();
                if extra.is_empty() {
                    println!("  Unclaimed: {}", style("none", "32", runtime));
                } else {
                    let unclaimed_str = extra.join(" ");
                    println!("  Unclaimed: {}", style(&unclaimed_str, "33", runtime));
                }
            }
            Err(error) => println!(
                "  Unclaimed: {}",
                style(&format!("unavailable ({error})"), "31", runtime)
            ),
        }
    }
}

fn print_flatpak_inventory(runtime: &Runtime, plan: &crate::types::Plan, filter: Option<&str>) {
    if filter.is_none_or(|f| f == "flatpak") && (!plan.flatpaks.is_empty() || filter.is_some()) {
        println!();
        println!("{}", style("Flatpaks", "36;1", runtime));
        let flatpaks_list = if plan.flatpaks.is_empty() {
            "none".to_string()
        } else {
            plan.flatpaks.iter().cloned().collect::<Vec<_>>().join(" ")
        };
        println!("  Declared Flatpaks: {flatpaks_list}");
        match installed_flatpaks() {
            Ok(installed) => {
                let extra: Vec<_> = installed
                    .difference(&plan.flatpaks)
                    .filter(|pkg| !plan.ignored_packages.contains(*pkg))
                    .cloned()
                    .collect();
                if extra.is_empty() {
                    println!("  Unclaimed Flatpaks: {}", style("none", "32", runtime));
                } else {
                    let unclaimed_str = extra.join(" ");
                    println!(
                        "  Unclaimed Flatpaks: {}",
                        style(&unclaimed_str, "33", runtime)
                    );
                }
            }
            Err(error) => println!(
                "  Unclaimed Flatpaks: {}",
                style(&format!("unavailable ({error})"), "31", runtime)
            ),
        }
    }
}

fn print_downloads_inventory(
    runtime: &Runtime,
    plan: &crate::types::Plan,
    filter: Option<&str>,
) -> Result<()> {
    if filter.is_some_and(|f| f != "downloads") {
        return Ok(());
    }
    println!("{}", style("Downloads", "36;1", runtime));
    let declared_downloads: BTreeSet<PathBuf> = plan
        .downloads
        .iter()
        .map(|download| download.display_path.clone())
        .collect();
    let managed_files: BTreeSet<PathBuf> = plan
        .files
        .iter()
        .map(|file| file.display_target.clone())
        .collect();
    for download in &plan.downloads {
        let state = if download.install_path.exists() {
            style("[present]", "32", runtime)
        } else {
            style("[missing]", "33", runtime)
        };
        println!(
            "  {} download {} -> {}",
            state,
            download.url_or_zip_url(),
            download.display_path.display()
        );
    }

    let mut unclaimed_downloads = Vec::new();
    for bin_dir in [
        runtime.home_dir.join(".local/bin"),
        runtime.resolve_abs_target(Path::new("/usr/local/bin")),
    ] {
        if !bin_dir.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&bin_dir)? {
            let entry = entry?;
            let path = entry.path();
            let display = if bin_dir.starts_with(&runtime.root_dir) {
                PathBuf::from("/").join(path.strip_prefix(&runtime.root_dir)?)
            } else {
                path.clone()
            };
            if path.is_file()
                && !declared_downloads.contains(&display)
                && !managed_files.contains(&display)
                && !crate::commands::lib::matches_any_glob(&display, &plan.ignored_files)
            {
                let _path_str = display.to_string_lossy();
                unclaimed_downloads.push(display);
            }
        }
    }
    if !unclaimed_downloads.is_empty() {
        println!("  Unclaimed Binaries:");
        for display in unclaimed_downloads {
            println!("    - {}", style(&display.to_string_lossy(), "33", runtime));
        }
    }
    Ok(())
}

fn print_services_inventory(runtime: &Runtime, plan: &crate::types::Plan, filter: Option<&str>) {
    if filter.is_some_and(|f| f != "services") {
        return;
    }
    println!();
    println!("{}", style("Services", "36;1", runtime));

    for scope in ["user", "system"] {
        println!("  {scope}:");
        let declared_units = plan.services.get(scope).cloned().unwrap_or_default();
        let declared_str = if declared_units.is_empty() {
            "none".to_string()
        } else {
            declared_units.iter().cloned().collect::<Vec<_>>().join(" ")
        };
        println!("    Declared Services: {declared_str}");

        let cmd_args: &[&str] = if scope == "user" {
            &[
                "systemctl",
                "--user",
                "list-unit-files",
                "--state=enabled",
                "--plain",
            ]
        } else {
            &["systemctl", "list-unit-files", "--state=enabled", "--plain"]
        };
        match crate::utils::command_lines(cmd_args) {
            Ok(lines) => {
                let enabled_units: BTreeSet<String> = lines
                    .into_iter()
                    .filter_map(|line| {
                        let mut parts = line.split_whitespace();
                        let unit = parts.next()?;
                        let state = parts.next()?;
                        if state == "enabled" {
                            Some(unit.to_string())
                        } else {
                            None
                        }
                    })
                    .collect();
                let extra: Vec<_> = enabled_units
                    .difference(&declared_units)
                    .filter(|unit| !plan.ignored_services.contains(*unit))
                    .cloned()
                    .collect();
                if extra.is_empty() {
                    println!(
                        "    Unclaimed Enabled Services: {}",
                        style("none", "32", runtime)
                    );
                } else {
                    println!(
                        "    Unclaimed Enabled Services: {}",
                        style(&extra.join(" "), "33", runtime)
                    );
                }
            }
            Err(error) => println!(
                "    Unclaimed Enabled Services: {}",
                style(&format!("unavailable ({error})"), "31", runtime)
            ),
        }
    }
}

pub fn run(runtime: &Runtime, filter: Option<&str>, format: &OutputFormat) -> Result<()> {
    let plan = build_plan(runtime, None)?;

    if !matches!(format, OutputFormat::Human) {
        let mut output = serde_json::Map::new();
        if filter.is_none_or(|value| value == "native") {
            let mut declared = Vec::new();
            let mut unclaimed = Vec::new();
            let mut errors = Vec::new();
            for (distro, packages) in &plan.packages {
                for package in packages {
                    declared.push(serde_json::json!({"manager": distro, "name": package}));
                }
                match installed_native_packages(distro) {
                    Ok(installed) => {
                        for package in installed.difference(packages) {
                            if !plan.ignored_packages.contains(package) {
                                unclaimed
                                    .push(serde_json::json!({"manager": distro, "name": package}));
                            }
                        }
                    }
                    Err(error) => errors.push(format!("{distro}: {error}")),
                }
            }
            output.insert(
                "native_packages".into(),
                serde_json::json!({
                    "declared": declared,
                    "unclaimed": unclaimed,
                    "errors": errors,
                }),
            );
        }
        if filter.is_none_or(|value| value == "flatpak") {
            let installed = installed_flatpaks();
            let (unclaimed, error) = match installed {
                Ok(installed) => (
                    installed
                        .difference(&plan.flatpaks)
                        .filter(|package| !plan.ignored_packages.contains(*package))
                        .cloned()
                        .collect::<Vec<_>>(),
                    String::new(),
                ),
                Err(error) => (Vec::new(), error.to_string()),
            };
            output.insert(
                "flatpaks".into(),
                serde_json::json!({
                    "declared": plan.flatpaks,
                    "unclaimed": unclaimed,
                    "error": error,
                }),
            );
        }
        if filter.is_none_or(|value| value == "downloads") {
            let declared: Vec<_> = plan
                .downloads
                .iter()
                .map(|download| {
                    serde_json::json!({
                        "url": download.url_or_zip_url(),
                        "path": download.display_path.display().to_string(),
                        "installed": download.install_path.exists(),
                    })
                })
                .collect();
            let declared_downloads: BTreeSet<PathBuf> = plan
                .downloads
                .iter()
                .map(|download| download.display_path.clone())
                .collect();
            let managed_files: BTreeSet<PathBuf> = plan
                .files
                .iter()
                .map(|file| file.display_target.clone())
                .collect();
            let mut unclaimed = Vec::new();
            for bin_dir in [
                runtime.home_dir.join(".local/bin"),
                runtime.resolve_abs_target(Path::new("/usr/local/bin")),
            ] {
                if !bin_dir.is_dir() {
                    continue;
                }
                for entry in fs::read_dir(&bin_dir)? {
                    let path = entry?.path();
                    let display = if bin_dir.starts_with(&runtime.root_dir) {
                        PathBuf::from("/").join(path.strip_prefix(&runtime.root_dir)?)
                    } else {
                        path.clone()
                    };
                    if path.is_file()
                        && !declared_downloads.contains(&display)
                        && !managed_files.contains(&display)
                        && !crate::commands::lib::matches_any_glob(&display, &plan.ignored_files)
                    {
                        unclaimed.push(display.to_string_lossy().into_owned());
                    }
                }
            }
            output.insert(
                "downloads".into(),
                serde_json::json!({
                    "declared": declared,
                    "unclaimed": unclaimed,
                }),
            );
        }
        if filter.is_none_or(|value| value == "services") {
            let mut declared = Vec::new();
            let mut unclaimed = Vec::new();
            let mut errors = Vec::new();
            for scope in ["user", "system"] {
                let units = plan.services.get(scope).cloned().unwrap_or_default();
                for unit in &units {
                    declared.push(serde_json::json!({"scope": scope, "unit": unit}));
                }
                let args: &[&str] = if scope == "user" {
                    &[
                        "systemctl",
                        "--user",
                        "list-unit-files",
                        "--state=enabled",
                        "--plain",
                    ]
                } else {
                    &["systemctl", "list-unit-files", "--state=enabled", "--plain"]
                };
                match crate::utils::command_lines(args) {
                    Ok(lines) => {
                        for line in lines {
                            let mut parts = line.split_whitespace();
                            let Some(unit) = parts.next() else { continue };
                            let Some("enabled") = parts.next() else {
                                continue;
                            };
                            if !units.contains(unit) && !plan.ignored_services.contains(unit) {
                                unclaimed.push(serde_json::json!({"scope": scope, "unit": unit}));
                            }
                        }
                    }
                    Err(error) => errors.push(format!("{scope}: {error}")),
                }
            }
            output.insert(
                "services".into(),
                serde_json::json!({
                    "declared": declared,
                    "unclaimed_enabled": unclaimed,
                    "errors": errors,
                }),
            );
        }
        return crate::utils::output::print_structured(format, &output);
    }

    print_native_packages_inventory(runtime, &plan, filter);
    print_flatpak_inventory(runtime, &plan, filter);

    println!();
    print_downloads_inventory(runtime, &plan, filter)?;

    print_services_inventory(runtime, &plan, filter);

    println!();
    println!(
        "{}",
        style(
            "Orphans never uninstalls; use this as an inventory report.",
            "30;1",
            runtime
        )
    );
    Ok(())
}
