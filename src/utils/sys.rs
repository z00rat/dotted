use color_eyre::eyre::{Result, WrapErr, bail};
use std::process::Command;

pub(crate) fn is_service_enabled(scope: &str, unit: &str) -> bool {
    let mut cmd = Command::new("systemctl");
    if scope == "user" {
        cmd.arg("--user");
    }
    cmd.args(["is-enabled", unit]);
    cmd.output().is_ok_and(|out| out.status.success())
}

pub(crate) fn is_service_active(scope: &str, unit: &str) -> bool {
    let mut cmd = Command::new("systemctl");
    if scope == "user" {
        cmd.arg("--user");
    }
    cmd.args(["is-active", unit]);
    cmd.output().is_ok_and(|out| out.status.success())
}

pub(crate) fn command_exists(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .output()
        .is_ok_and(|out| out.status.success())
}

pub(crate) fn command_lines(args: &[&str]) -> Result<std::collections::BTreeSet<String>> {
    let (cmd, rest) = args
        .split_first()
        .ok_or_else(|| color_eyre::eyre::eyre!("empty command"))?;
    let output = Command::new(cmd)
        .args(rest)
        .output()
        .wrap_err_with(|| format!("failed to run {cmd}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{cmd} failed: {stderr}");
    }
    let text = String::from_utf8(output.stdout)?;
    Ok(text
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect())
}

pub(crate) fn native_package_command(
    distro: &str,
    missing: &std::collections::BTreeSet<String>,
    package_commands: &std::collections::HashMap<String, String>,
) -> Result<String> {
    let prefix = package_commands
        .get(distro)
        .ok_or_else(|| color_eyre::eyre::eyre!("unsupported distro {distro}"))?;
    let packages = missing
        .iter()
        .map(|package| shell_escape::escape(package.into()).to_string())
        .collect::<Vec<_>>()
        .join(" ");
    Ok(format!("{prefix} {packages}"))
}

pub(crate) fn is_package_installed(distro: &str, package: &str) -> bool {
    match distro {
        "archlinux" => Command::new("pacman")
            .args(["-Qq", package])
            .output()
            .is_ok_and(|out| out.status.success()),
        "ubuntu" => Command::new("dpkg-query")
            .args(["-W", "-f=${Status}", package])
            .output()
            .is_ok_and(|out| {
                out.status.success()
                    && String::from_utf8_lossy(&out.stdout).contains("install ok installed")
            }),
        "fedora" => Command::new("rpm")
            .args(["-q", package])
            .output()
            .is_ok_and(|out| out.status.success()),
        _ => false,
    }
}

pub(crate) fn is_flatpak_installed(package: &str) -> bool {
    Command::new("flatpak")
        .args(["info", package])
        .output()
        .is_ok_and(|out| out.status.success())
}

pub(crate) fn shell_join(args: &[String]) -> String {
    args.iter()
        .map(|arg| {
            if arg.contains(' ') || arg.contains('"') {
                format!("{arg:?}")
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
