use color_eyre::eyre::{Result, WrapErr, bail};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::types::{DownloadSource, Plan, PlannedFile, Runtime};
use crate::utils::{
    backup_file, confirm, native_package_command, preserve_source_permissions, show_file_diff,
    style,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConflictAction {
    Deploy,
    Keep,
    UpdateRepository,
    Abort,
}

fn resolve_conflict(runtime: &Runtime, file: &PlannedFile) -> Result<ConflictAction> {
    if runtime.no_color {
        loop {
            print!(
                "Conflict in {}. [r]ight (deploy new), [l]eft (keep current), [u]pdate repository, [a]bort? ",
                file.display_target.display()
            );
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            match line.trim().to_ascii_lowercase().as_str() {
                "r" | "right" => return Ok(ConflictAction::Deploy),
                "l" | "left" => return Ok(ConflictAction::Keep),
                "u" | "update" | "repo" => return Ok(ConflictAction::UpdateRepository),
                "a" | "abort" => return Ok(ConflictAction::Abort),
                _ => println!("invalid choice. Please enter 'r', 'l', 'u', or 'a'."),
            }
        }
    }

    cliclack::select(format!(
        "Conflict while writing {}",
        file.display_target.display()
    ))
    .item(
        ConflictAction::Deploy,
        "Right (deploy new / overwrite target)",
        "",
    )
    .item(
        ConflictAction::Keep,
        "Left (keep current target / skip)",
        "",
    )
    .item(
        ConflictAction::UpdateRepository,
        "Update repository (overwrite the repository file with the target)",
        "",
    )
    .item(ConflictAction::Abort, "Abort deployment", "")
    .interact()
    .map_err(|e| color_eyre::eyre::Report::msg(e.to_string()))
}

pub(crate) fn write_file_as_root(target: &Path, bytes: &[u8]) -> Result<()> {
    let mut child = Command::new("sudo")
        .args(["tee", &target.to_string_lossy()])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .wrap_err_with(|| format!("start sudo for {}", target.display()))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(bytes)?;
    }
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        bail!("failed to write as root to {}", target.display())
    }
}

pub(crate) fn apply_file(runtime: &Runtime, file: &PlannedFile, yes: bool) -> Result<()> {
    if let Some(parent) = file.target.parent() {
        fs::create_dir_all(parent)?;
    }
    if file.target.exists() {
        let current = fs::read(&file.target)
            .wrap_err_with(|| format!("read current file {}", file.display_target.display()))?;
        if current == file.bytes {
            println!(
                "{} {}",
                style("same", "32", runtime),
                file.display_target.display()
            );
            return Ok(());
        }
        if !yes {
            show_file_diff(file, &current, runtime);
            match resolve_conflict(runtime, file)? {
                ConflictAction::Deploy => {}
                ConflictAction::Keep => {
                    println!(
                        "{} {}",
                        style("skip", "33", runtime),
                        file.display_target.display()
                    );
                    return Ok(());
                }
                ConflictAction::UpdateRepository => {
                    fs::write(&file.source, &current)?;
                    println!(
                        "{} {}",
                        style("updated repository", "32", runtime),
                        runtime.display_path(&file.source).display()
                    );
                    return Ok(());
                }
                ConflictAction::Abort => {
                    bail!(
                        "aborted by user due to conflict in {}",
                        file.display_target.display()
                    )
                }
            }
            if !runtime.no_color {
                print!("\x1B[2J\x1B[H");
                let _ = std::io::stdout().flush();
            }
        }
        backup_file(runtime, &file.target, &file.display_target)?;
    } else if let (false, Some(text)) = (yes, &file.text) {
        crate::utils::print_new_file_content(&file.display_target.to_string_lossy(), text, runtime);
    }

    if let Err(error) = fs::write(&file.target, &file.bytes) {
        if error.kind() == std::io::ErrorKind::PermissionDenied {
            println!("Permission denied for {}.", file.display_target.display());
            if yes
                || confirm(
                    "Attempt to write as root with sudo? [y/N] ",
                    runtime.no_color,
                )?
            {
                write_file_as_root(&file.target, &file.bytes)?;
            } else {
                return Err(error).wrap_err(format!("write {}", file.display_target.display()));
            }
        } else {
            return Err(error).wrap_err(format!("write {}", file.display_target.display()));
        }
    }
    preserve_source_permissions(&file.source, &file.target)?;
    println!(
        "{} {}",
        style("wrote", "32;1", runtime),
        file.display_target.display()
    );
    Ok(())
}

pub(crate) fn apply_packages_and_downloads(
    runtime: &Runtime,
    plan: &Plan,
    _yes: bool,
) -> Result<bool> {
    let mut commands = Vec::new();
    let dotted = crate::commands::lib::load_dotted(runtime)?;
    let package_commands = &dotted.config.package_commands;

    for (distro, packages) in &plan.packages {
        let missing: BTreeSet<_> = packages
            .iter()
            .filter(|package| !crate::utils::is_package_installed(distro, package))
            .cloned()
            .collect();
        if !missing.is_empty() {
            let command = native_package_command(distro, &missing, package_commands)?;
            let command = crate::utils::shell_join(&command);
            println!("native packages ({distro}): {command}");
            commands.push(command);
        }
    }

    let missing_flatpaks: BTreeSet<_> = plan
        .flatpaks
        .iter()
        .filter(|flatpak| !crate::utils::is_flatpak_installed(flatpak))
        .cloned()
        .collect();
    if !missing_flatpaks.is_empty() {
        let mut command = vec![
            "flatpak".to_string(),
            "install".to_string(),
            "-y".to_string(),
        ];
        command.extend(missing_flatpaks);
        let command = crate::utils::shell_join(&command);
        println!("flatpaks: {command}");
        commands.push(command);
    }

    for download in &plan.downloads {
        if download.install_path.exists() {
            continue;
        }
        let command = match &download.source {
            DownloadSource::Url(url) => format!(
                "curl --fail --location --output {} {}",
                download.display_path.display(),
                url
            ),
            DownloadSource::Zip { url, path } => format!(
                "curl --fail --location --output archive.zip {} && unzip -p archive.zip {} > {}",
                url,
                path,
                download.display_path.display()
            ),
        };
        println!("download: {command}");
        commands.push(command);
    }

    for (scope, service_set) in &plan.services {
        let to_enable: Vec<_> = service_set
            .iter()
            .filter(|unit| !crate::utils::is_service_enabled(scope, unit))
            .cloned()
            .collect();
        if !to_enable.is_empty() {
            let prefix = if scope == "user" {
                "systemctl --user enable --now"
            } else {
                "sudo systemctl enable --now"
            };
            let command = format!("{prefix} {}", to_enable.join(" "));
            println!("services ({scope}): {command}");
            commands.push(command);
        }
    }

    if !commands.is_empty() {
        println!();
        println!(
            "{}",
            style(
                "COMMANDS PLANNED/EXECUTED FOR PACKAGES/DOWNLOADS/SERVICES:",
                "36;1",
                runtime
            )
        );
        for command in &commands {
            println!("  {}", style(command, "33", runtime));
        }
    }
    Ok(!commands.is_empty())
}

pub(crate) fn write_env_file(runtime: &Runtime, plan: &Plan) -> Result<()> {
    let dotted = crate::commands::lib::load_dotted(runtime)?;
    let mut content = String::new();
    for (key, value) in &plan.env {
        let _ = writeln!(
            content,
            "export {key}={}",
            shell_escape::escape(value.into())
        );
    }
    for env_path in &dotted.config.env_path {
        let path = runtime.resolve_tilde(env_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, &content)
            .wrap_err_with(|| format!("write environment file {}", path.display()))?;
    }
    Ok(())
}
