use color_eyre::eyre::{Result, anyhow, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::{DirEntry, WalkDir};

use crate::plan::{normalize_arch, plan_download};
use crate::types::{
    ABOUT_TOML, AboutEntry, AboutFile, Artifact, DottedFile, Plan, RepoConfig, Runtime,
    SETTINGS_DIR,
};
use crate::utils::{command_lines, run_git, style};

pub(crate) fn load_dotted(runtime: &Runtime) -> Result<DottedFile> {
    let mut dotted: DottedFile = crate::types::read_toml(&runtime.dotted_path())?;
    for (distro, command) in crate::types::dotted_file::default_package_commands() {
        dotted
            .config
            .package_commands
            .entry(distro)
            .or_insert(command);
    }
    for color in [
        &dotted.color.success,
        &dotted.color.warning,
        &dotted.color.error,
        &dotted.color.info,
        &dotted.color.muted,
        &dotted.color.installed,
        &dotted.color.diff,
    ] {
        if !crate::utils::is_terminal_color(color) {
            color_eyre::eyre::bail!(
                "invalid terminal color `{color}`; use a standard or bright ANSI color name"
            );
        }
    }
    Ok(dotted)
}

pub(crate) fn settings_path(runtime: &Runtime) -> PathBuf {
    runtime
        .settings_root()
        .join(&runtime.device)
        .join(format!("{}.toml", runtime.user))
}

pub(crate) fn repository_path(runtime: &Runtime, repo: &str) -> PathBuf {
    runtime.dotted_dir.join(if repo == "artifacts" {
        crate::types::ARTIFACTS_DIR
    } else {
        repo
    })
}

pub(crate) fn configured_repos(dotted: &DottedFile) -> Vec<RepoConfig> {
    dotted.repos.clone()
}

pub(crate) fn checkout_repo(path: &Path, repo: &RepoConfig) -> Result<()> {
    if let Some(branch) = &repo.branch {
        run_git(path, ["checkout", branch.as_str()])?;
    }
    if let Some(tag) = &repo.tag {
        run_git(path, ["checkout", tag.as_str()])?;
    }
    if let Some(revision) = &repo.revision {
        run_git(path, ["checkout", revision.as_str()])?;
    }
    Ok(())
}

pub(crate) fn commit_and_push(path: &Path, message: &str) -> Result<()> {
    if !path.join(".git").exists() {
        println!("Skipping non-Git repository {}", path.display());
        return Ok(());
    }
    run_git(path, ["add", "."])?;
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(path)
        .output()?;
    if status.stdout.is_empty() {
        println!("Nothing to commit in {}", path.display());
        return Ok(());
    }
    run_git(path, ["commit", "-m", message])?;
    let remotes = Command::new("git")
        .args(["remote"])
        .current_dir(path)
        .output()?;
    if !remotes.stdout.is_empty() {
        run_git(path, ["push"])?;
    }
    Ok(())
}

fn show_section(filter: Option<&str>, section: &str) -> bool {
    filter.is_none_or(|value| value == section)
}

fn print_plan_environment(plan: &Plan, filter: Option<&str>) {
    if show_section(filter, "env") && !plan.env.is_empty() {
        println!();
        println!("env:");
        for (key, value) in &plan.env {
            println!("  {key} = \"{value}\"");
        }
    }
    if !plan.env_overrides.is_empty() {
        println!("env overrides: {}", plan.env_overrides.join(", "));
    }
}

fn print_plan_packages(runtime: &Runtime, plan: &Plan) {
    for (distro, packages) in &plan.packages {
        for package in packages {
            let (label, color) = if crate::utils::is_package_installed(distro, package) {
                ("[installed]", "34")
            } else {
                ("[missing]", "32")
            };
            let status = style(&format!("{label:<11}"), color, runtime);
            println!("  {status} native {package}");
        }
    }
}

fn print_plan_downloads(runtime: &Runtime, plan: &Plan) {
    for flatpak in &plan.flatpaks {
        let (label, color) = if crate::utils::is_flatpak_installed(flatpak) {
            ("[installed]", "34")
        } else {
            ("[missing]", "32")
        };
        let status = style(&format!("{label:<11}"), color, runtime);
        println!("  {status} flatpak             {flatpak}");
    }
    for download in &plan.downloads {
        let (label, color) = if download.install_path.exists() {
            ("[installed]", "34")
        } else {
            ("[missing]", "32")
        };
        let status = style(&format!("{label:<11}"), color, runtime);
        println!(
            "  {status} download  {} -> {}",
            download.url_or_zip_url(),
            download.display_path.display()
        );
    }
}

fn print_plan_services(runtime: &Runtime, plan: &Plan) {
    for (scope, service_set) in &plan.services {
        for unit in service_set {
            let status = match (
                crate::utils::is_service_enabled(scope, unit),
                crate::utils::is_service_active(scope, unit),
            ) {
                (true, true) => style("[active & enabled]", "34", runtime),
                (true, false) => style("[inactive & enabled]", "33", runtime),
                (false, true) => style("[active & disabled]", "33", runtime),
                (false, false) => style("[disabled]", "32", runtime),
            };
            println!("  {status} service ({scope}) {unit}");
        }
    }
}

pub(crate) fn print_plan_extras(runtime: &Runtime, plan: &Plan, filter: Option<&str>) {
    print_plan_environment(plan, filter);
    let packages = show_section(filter, "packages") && !plan.packages.is_empty();
    let downloads = show_section(filter, "downloads")
        && (!plan.flatpaks.is_empty() || !plan.downloads.is_empty());
    let services = show_section(filter, "services") && !plan.services.is_empty();
    if !(packages || downloads || services) {
        return;
    }
    println!();
    println!(
        "{}",
        if services && !packages && !downloads {
            "services:"
        } else {
            "packages/downloads/services:"
        }
    );
    if packages {
        print_plan_packages(runtime, plan);
    }
    if downloads {
        print_plan_downloads(runtime, plan);
    }
    if services {
        print_plan_services(runtime, plan);
    }
}

pub(crate) fn split_artifact_id(id: &str) -> Result<(&str, &str)> {
    if let Some(name) = id.strip_prefix('/') {
        if name.is_empty() || name.contains('/') {
            bail!("Artifact ID must be /artifact or repository/artifact.");
        }
        return Ok(("artifacts", name));
    }
    let (repo, artifact) = id
        .split_once('/')
        .ok_or_else(|| anyhow!("Artifact ID must be /artifact or repository/artifact."))?;
    if repo.is_empty() || artifact.is_empty() {
        bail!("Artifact ID must be /artifact or repository/artifact.");
    }
    Ok((repo, artifact))
}

pub(crate) fn artifact_relative_from_system_path(runtime: &Runtime, source: &Path) -> PathBuf {
    let absolute = fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
    if let Ok(rest) = absolute.strip_prefix(&runtime.home_dir) {
        return PathBuf::from("home").join(rest);
    }
    let relative = absolute.strip_prefix("/").unwrap_or(&absolute);
    relative.to_path_buf()
}

pub(crate) fn ensure_about_entry(runtime: &Runtime, repo: &str, artifact: &str) -> Result<()> {
    let path = repository_path(runtime, repo).join(ABOUT_TOML);
    let mut about: AboutFile = if path.exists() {
        crate::types::read_toml(&path)?
    } else {
        AboutFile::default()
    };
    about
        .about
        .entry(artifact.to_string())
        .or_insert(AboutEntry {
            r: 1,
            description: String::new(),
        });
    crate::types::write_toml(&path, &about)
}

pub(crate) fn matches_any_glob(path: &Path, patterns: &BTreeSet<PathBuf>) -> bool {
    let path_str = path.to_string_lossy();
    for pattern in patterns {
        let pattern_str = pattern.to_string_lossy();
        if glob::Pattern::new(&pattern_str).is_ok_and(|p| p.matches(&path_str)) {
            return true;
        }
    }
    false
}

pub(crate) fn is_ignored_dir(entry: &DirEntry, ignored_dirs: &BTreeSet<PathBuf>) -> bool {
    entry.file_type().is_dir() && ignored_dirs.contains(entry.path())
}

pub(crate) fn installed_native_packages(distro: &str) -> Result<BTreeSet<String>> {
    let command: &[&str] = match distro {
        "archlinux" => &["pacman", "-Qqe"],
        "fedora" => &["dnf", "repoquery", "--userinstalled", "--qf", "%{name}"],
        "ubuntu" => &["apt-mark", "showmanual"],
        other => bail!("Unsupported package distribution: {other}"),
    };
    command_lines(command)
}

pub(crate) fn installed_flatpaks() -> Result<BTreeSet<String>> {
    command_lines(&["flatpak", "list", "--app", "--columns=application"][..])
}

pub(crate) type Claims = (
    BTreeMap<String, BTreeSet<String>>,
    BTreeSet<String>,
    BTreeSet<PathBuf>,
    BTreeMap<String, BTreeSet<String>>,
);

pub(crate) fn collect_claims(runtime: &Runtime, artifacts: &[Artifact]) -> Result<Claims> {
    let distro = runtime.distro.clone();
    let arch = normalize_arch(env::consts::ARCH);
    let mut packages: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut flatpaks = BTreeSet::new();
    let mut downloads = BTreeSet::new();
    let mut services: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for artifact in artifacts {
        if let Some(set) = artifact.bin.distro.get(&distro) {
            packages
                .entry(distro.clone())
                .or_default()
                .extend(set.packages.iter().cloned());
        }
        flatpaks.extend(artifact.bin.flatpak.packages.iter().cloned());
        for (scope, service_set) in &artifact.bin.services {
            services
                .entry(scope.clone())
                .or_default()
                .extend(service_set.units.iter().cloned());
        }
        if let Some(download) = artifact.bin.download.get(&arch) {
            downloads.insert(plan_download(runtime, &artifact.id, &arch, download)?.display_path);
        }
    }
    Ok((packages, flatpaks, downloads, services))
}

pub(crate) fn print_unclaimed_hints(runtime: &Runtime, target: &Claims, other: &Claims) {
    for (distro, packages) in &target.0 {
        let other_packages = other.0.get(distro).cloned().unwrap_or_default();
        let unclaimed: Vec<_> = packages.difference(&other_packages).cloned().collect();
        let installed: Vec<_> = unclaimed
            .into_iter()
            .filter(|package| crate::utils::is_package_installed(distro, package))
            .collect();
        if !installed.is_empty() {
            println!(
                "Unclaimed native packages ({distro}): {}",
                installed.join(" ")
            );
        }
    }
    let unclaimed_flatpaks: Vec<_> = target.1.difference(&other.1).cloned().collect();
    let installed_flatpaks: Vec<_> = unclaimed_flatpaks
        .into_iter()
        .filter(|package| crate::utils::is_flatpak_installed(package))
        .collect();
    if !installed_flatpaks.is_empty() {
        println!(
            "Unclaimed Flatpak packages: {}",
            installed_flatpaks.join(" ")
        );
    }
    let unclaimed_downloads: Vec<_> = target.2.difference(&other.2).collect();
    for path in unclaimed_downloads {
        if path.exists() {
            println!(
                "Unclaimed download: {}",
                runtime.display_path(path).display()
            );
        }
    }
    for (scope, units) in &target.3 {
        let other_units = other.3.get(scope).cloned().unwrap_or_default();
        let unclaimed: Vec<_> = units.difference(&other_units).cloned().collect();
        let active_or_enabled: Vec<_> = unclaimed
            .into_iter()
            .filter(|unit| {
                crate::utils::is_service_enabled(scope, unit)
                    || crate::utils::is_service_active(scope, unit)
            })
            .collect();
        if !active_or_enabled.is_empty() {
            let cmd_prefix = if scope == "user" {
                "systemctl --user disable --now"
            } else {
                "sudo systemctl disable --now"
            };
            println!(
                "unclaimed service ({scope}): {cmd_prefix} {}",
                active_or_enabled.join(" ")
            );
        }
    }
}

pub(crate) fn restore_one(source: &Path, target: &Path) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, target)?;
    println!("Restored {}", target.display());
    Ok(())
}

pub(crate) fn control_toml_paths(runtime: &Runtime) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in WalkDir::new(&runtime.dotted_dir) {
        let entry = entry?;
        if entry.file_type().is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "toml")
        {
            paths.push(entry.path().to_path_buf());
        }
    }
    paths.sort();
    Ok(paths)
}

pub(crate) fn artifact_dirs_without_about(
    runtime: &Runtime,
) -> Result<Vec<(String, String, PathBuf)>> {
    let mut missing = Vec::new();
    if !runtime.dotted_dir.exists() {
        return Ok(missing);
    }
    let dotted = load_dotted(runtime)?;
    let configured: BTreeSet<String> = dotted.repos.into_iter().map(|repo| repo.name).collect();
    for repo_entry in fs::read_dir(&runtime.dotted_dir)? {
        let repo_entry = repo_entry?;
        let repo_path = repo_entry.path();
        if !repo_path.is_dir() || repo_entry.file_name() == std::ffi::OsStr::new(SETTINGS_DIR) {
            continue;
        }
        let repo_name = repo_entry.file_name().to_string_lossy().to_string();
        if repo_name != crate::types::ARTIFACTS_DIR && !configured.contains(&repo_name) {
            continue;
        }
        let about_path = repo_path.join(ABOUT_TOML);
        if !about_path.exists() {
            continue;
        }
        let about: AboutFile = crate::types::read_toml(&about_path)?;
        for artifact_entry in fs::read_dir(&repo_path)? {
            let artifact_entry = artifact_entry?;
            let artifact_path = artifact_entry.path();
            if !artifact_path.is_dir() {
                continue;
            }
            let name = artifact_entry.file_name().to_string_lossy().to_string();
            if !about.about.contains_key(&name) {
                missing.push((
                    repo_entry.file_name().to_string_lossy().to_string(),
                    name,
                    artifact_path,
                ));
            }
        }
    }
    Ok(missing)
}

pub(crate) fn check_remote_reachability(
    runtime: &Runtime,
    dir: &Path,
    label: &str,
    problems: &mut usize,
    filter: Option<&str>,
) {
    if !dir.join(".git").exists() {
        return;
    }
    let matches_filter = |s: &str| -> bool {
        if let Some(f) = filter {
            s.to_lowercase().contains(&f.to_lowercase())
        } else {
            true
        }
    };
    let log = |prefix: &str, text: &str| {
        let line = format!("{prefix} {text}");
        if matches_filter(&line) {
            println!("{line}");
        }
    };

    match Command::new("git")
        .args(["remote"])
        .current_dir(dir)
        .output()
    {
        Ok(out) => {
            let remotes = String::from_utf8_lossy(&out.stdout);
            for remote in remotes.lines().map(str::trim).filter(|r| !r.is_empty()) {
                log(
                    "",
                    &format!("checking remote reachability for {label} ({remote})..."),
                );
                let check = Command::new("git")
                    .args(["ls-remote", "--exit-code", "--heads", remote])
                    .current_dir(dir)
                    .status();
                match check {
                    Ok(status) if status.success() => {
                        log(
                            &style("ok", "32", runtime),
                            &format!("remote {label} ({remote})"),
                        );
                    }
                    _ => {
                        *problems += 1;
                        log(
                            &style("bad", "31", runtime),
                            &format!("remote reachability for {label} ({remote})"),
                        );
                    }
                }
            }
        }
        Err(e) => {
            *problems += 1;
            log("", &format!("failed to check remotes for {label}: {e}"));
        }
    }
}

pub(crate) fn check_tool(
    runtime: &Runtime,
    tool: &str,
    problems: &mut usize,
    filter: Option<&str>,
) {
    let matches_filter = |s: &str| -> bool {
        if let Some(f) = filter {
            s.to_lowercase().contains(&f.to_lowercase())
        } else {
            true
        }
    };
    let log = |prefix: &str, text: &str| {
        let line = format!("{prefix} {text}");
        if matches_filter(&line) {
            println!("{line}");
        }
    };

    if crate::utils::command_exists(tool) {
        log(&style("ok", "32", runtime), &format!("tool {tool}"));
    } else {
        *problems += 1;
        log(&style("missing", "31", runtime), &format!("tool {tool}"));
    }
}
