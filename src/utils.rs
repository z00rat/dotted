use crate::types::{PlannedFile, Runtime};
use blake2::{Blake2b512, Digest as BlakeDigest};
use chrono::Utc;
use color_eyre::eyre::{Result, WrapErr, bail};
use similar::{ChangeTag, TextDiff};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

pub(crate) fn blake2b_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut hasher = Blake2b512::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .fold(String::new(), |mut acc, byte| {
            let _ = write!(acc, "{byte:02x}");
            acc
        })
}

pub(crate) fn run_git<const N: usize>(dir: &Path, args: [&str; N]) -> Result<()> {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .wrap_err_with(|| format!("run git in {}", dir.display()))?;
    if status.success() {
        Ok(())
    } else {
        bail!("git failed in {}", dir.display())
    }
}

pub(crate) fn confirm(prompt: &str, no_color: bool) -> Result<bool> {
    if no_color {
        print!("{prompt}");
        io::stdout().flush()?;
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        let answer = line.trim().to_ascii_lowercase();
        Ok(answer.is_empty() || answer == "y" || answer == "yes")
    } else {
        cliclack::confirm(prompt)
            .interact()
            .map_err(|e| color_eyre::eyre::Report::msg(e.to_string()))
    }
}

pub(crate) fn preserve_source_permissions(source: &Path, target: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(source)?.permissions().mode();
        let mut permissions = fs::metadata(target)?.permissions();
        permissions.set_mode(mode);
        if let Err(e) = fs::set_permissions(target, permissions) {
            if e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(1) {
                let status = Command::new("sudo")
                    .args([
                        "chmod",
                        &format!("{:o}", mode & 0o777),
                        &target.to_string_lossy(),
                    ])
                    .status()?;
                if !status.success() {
                    bail!("failed to set permissions as root on {}", target.display());
                }
            } else {
                return Err(e.into());
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (source, target);
    }
    Ok(())
}

pub(crate) fn backup_file(runtime: &Runtime, target: &Path, display_target: &Path) -> Result<()> {
    let relative = display_target.strip_prefix("/").unwrap_or(display_target);
    let backup = runtime
        .backup_root()
        .join(Utc::now().timestamp().to_string())
        .join(relative);
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(target, backup)?;
    Ok(())
}

pub(crate) fn style(text: &str, color_code: &str, runtime: &Runtime) -> String {
    if runtime.no_color {
        text.to_string()
    } else {
        format!("\x1b[{color_code}m{text}\x1b[0m")
    }
}

pub(crate) fn is_terminal_color(value: &str) -> bool {
    matches!(
        value,
        "black"
            | "red"
            | "green"
            | "yellow"
            | "blue"
            | "magenta"
            | "cyan"
            | "white"
            | "bright-black"
            | "bright-red"
            | "bright-green"
            | "bright-yellow"
            | "bright-blue"
            | "bright-magenta"
            | "bright-cyan"
            | "bright-white"
    )
}

pub(crate) fn print_line_diff(
    left_title: &str,
    right_title: &str,
    left_text: &str,
    right_text: &str,
    runtime: &Runtime,
) {
    let diff = TextDiff::from_lines(left_text, right_text);
    println!(
        "--- {}\n+++ {}",
        style(left_title, "31;1", runtime),
        style(right_title, "32;1", runtime)
    );
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Delete => {
                print!("-{}", style(change.value(), "31", runtime));
            }
            ChangeTag::Insert => {
                print!("+{}", style(change.value(), "32", runtime));
            }
            ChangeTag::Equal => {
                print!(" {}", change.value());
            }
        }
    }
}

pub(crate) fn print_new_file_content(title: &str, text: &str, runtime: &Runtime) {
    let col_width = 78;
    let hdr = format!(" [NEW FILE] {title:<col_width$} ");
    println!("{}", style(&hdr, "32;1", runtime));
    let border = "━".repeat(col_width);
    println!("{}", style(&border, "36", runtime));
    for line in text.lines() {
        println!("+ {}", style(line, "32", runtime));
    }
}

pub(crate) fn show_file_diff(file: &PlannedFile, current: &[u8], runtime: &Runtime) {
    if let (Ok(old), Some(new)) = (String::from_utf8(current.to_vec()), &file.text) {
        print_line_diff("Current (on-disk)", "New (planned)", &old, new, runtime);
    } else {
        println!(
            "{}",
            style(
                &format!(
                    "binary differs {}: current b2:{} planned b2:{}",
                    file.display_target.display(),
                    blake2b_hex(current),
                    blake2b_hex(&file.bytes)
                ),
                "31",
                runtime
            )
        );
    }
}

pub(crate) fn command_exists(tool: &str) -> bool {
    env::var_os("PATH").is_some_and(|path| {
        env::split_paths(&path).any(|dir| {
            let candidate = dir.join(tool);
            candidate.is_file()
        })
    })
}

pub(crate) fn command_lines(command: &[&str]) -> Result<BTreeSet<String>> {
    let Some((program, args)) = command.split_first() else {
        return Ok(BTreeSet::new());
    };
    if !command_exists(program) {
        bail!("{program} not found");
    }
    let output = Command::new(program).args(args).output()?;
    if !output.status.success() {
        bail!("{program} failed");
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

pub(crate) fn native_package_command(
    distro: &str,
    packages: &BTreeSet<String>,
    package_commands: &std::collections::HashMap<String, String>,
) -> Result<Vec<String>> {
    let cmd = package_commands
        .get(distro)
        .ok_or_else(|| color_eyre::eyre::eyre!("unsupported package distro: {distro}"))?;
    let mut command: Vec<String> = cmd.split_whitespace().map(String::from).collect();
    command.extend(packages.iter().cloned());
    Ok(command)
}

pub(crate) fn shell_join(command: &[String]) -> String {
    command
        .iter()
        .map(|part| shell_escape::escape(part.into()).to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn is_package_installed(distro: &str, package: &str) -> bool {
    match distro {
        "archlinux" => Command::new("pacman")
            .args(["-Qq", package])
            .output()
            .is_ok_and(|out| out.status.success()),
        "fedora" => Command::new("rpm")
            .args(["-q", package])
            .output()
            .is_ok_and(|out| out.status.success()),
        "ubuntu" => Command::new("dpkg")
            .args(["-s", package])
            .output()
            .is_ok_and(|out| {
                if out.status.success() {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    stdout.contains("Status: install ok installed")
                } else {
                    false
                }
            }),
        _ => false,
    }
}

pub(crate) fn is_flatpak_installed(package: &str) -> bool {
    Command::new("flatpak")
        .args(["info", package])
        .output()
        .is_ok_and(|out| out.status.success())
}

pub(crate) fn print_banner(title: &str, runtime: &Runtime) {
    let _ = (title, runtime);
}
