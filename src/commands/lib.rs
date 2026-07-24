use color_eyre::eyre::{Result, WrapErr, anyhow, bail};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::{DirEntry, WalkDir};

use crate::plan::{normalize_arch, plan_download};
use crate::types::{
    ABOUT_TOML, AboutEntry, AboutFile, Artifact, DottedFile, Plan, PlannedFile, RepoConfig,
    Runtime, SETTINGS_DIR,
};
use crate::utils::{
    backup_file, command_lines, confirm, native_package_command, preserve_source_permissions,
    run_git, show_file_diff, style,
};

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
        println!("skip non-git repo {}", path.display());
        return Ok(());
    }
    run_git(path, ["add", "."])?;
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(path)
        .output()?;
    if status.stdout.is_empty() {
        println!("nothing to commit in {}", path.display());
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

pub(crate) fn print_plan_extras(runtime: &Runtime, plan: &Plan, filter: Option<&str>) {
    let show = |section: &str| filter.is_none_or(|f| f == section);

    if show("env") && !plan.env.is_empty() {
        let env_keys: Vec<_> = plan.env.keys().collect();
        if !env_keys.is_empty() {
            println!();
            println!("env:");
            for key in env_keys {
                if let Some(val) = plan.env.get(key) {
                    println!("  {key} = \"{val}\"");
                }
            }
        }
    }
    if !plan.env_overrides.is_empty() {
        let overrides = plan.env_overrides.clone();
        if !overrides.is_empty() {
            println!("env overrides: {}", overrides.join(", "));
        }
    }
    if (show("packages") && !plan.packages.is_empty())
        || (show("downloads") && (!plan.flatpaks.is_empty() || !plan.downloads.is_empty()))
    {
        println!();
        println!("packages/downloads:");
        if show("packages") {
            for (distro, packages) in &plan.packages {
                for pkg in packages {
                    let installed = crate::utils::is_package_installed(distro, pkg);
                    let (prefix, color) = if installed {
                        ("[installed]", "34") // Blue (highly visible)
                    } else {
                        ("[missing]", "32") // Green
                    };
                    let padded_prefix = format!("{prefix:<11}");
                    let status_tag = style(&padded_prefix, color, runtime);
                    println!("  {status_tag} native {pkg}");
                }
            }
        }
        if show("downloads") {
            for flatpak in &plan.flatpaks {
                let installed = crate::utils::is_flatpak_installed(flatpak);
                let (prefix, color) = if installed {
                    ("[installed]", "34")
                } else {
                    ("[missing]", "32")
                };
                let padded_prefix = format!("{prefix:<11}");
                let status_tag = style(&padded_prefix, color, runtime);
                println!("  {status_tag} flatpak             {flatpak}");
            }
            for download in &plan.downloads {
                let installed = download.install_path.exists();
                let (prefix, color) = if installed {
                    ("[installed]", "34")
                } else {
                    ("[missing]", "32")
                };
                let padded_prefix = format!("{prefix:<11}");
                let status_tag = style(&padded_prefix, color, runtime);
                let url = download.url_or_zip_url();
                println!(
                    "  {} download  {} -> {}",
                    status_tag,
                    url,
                    download.display_path.display()
                );
            }
        }
    }
}

pub(crate) fn write_file_as_root(target: &Path, bytes: &[u8]) -> Result<()> {
    let mut child = Command::new("sudo")
        .args(["tee", &target.to_string_lossy()])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()?;
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

fn resolve_conflict(runtime: &Runtime, file: &PlannedFile) -> Result<&'static str> {
    if runtime.no_color {
        loop {
            print!(
                "Conflict in {}! [r]ight (deploy new), [l]eft (keep current), [a]bort? ",
                file.display_target.display()
            );
            std::io::Write::flush(&mut std::io::stdout())?;
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
            let choice = line.trim().to_ascii_lowercase();
            match choice.as_str() {
                "r" | "right" => return Ok("right"),
                "l" | "left" => return Ok("left"),
                "a" | "abort" => return Ok("abort"),
                _ => {
                    println!("invalid choice. Please enter 'r', 'l', or 'a'.");
                }
            }
        }
    } else {
        cliclack::select(format!(
            "Conflict in file write for {}",
            file.display_target.display()
        ))
        .item("right", "Right (deploy new / overwrite)", "")
        .item("left", "Left (keep current / skip)", "")
        .item("abort", "Abort deployment", "")
        .interact()
        .map_err(|e| color_eyre::eyre::Report::msg(e.to_string()))
    }
}

pub(crate) fn apply_file(runtime: &Runtime, file: &PlannedFile, yes: bool) -> Result<()> {
    if let Some(parent) = file.target.parent() {
        fs::create_dir_all(parent)?;
    }
    if file.target.exists() {
        let current = fs::read(&file.target)?;
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
            let action = resolve_conflict(runtime, file)?;

            if !runtime.no_color {
                print!("\x1B[2J\x1B[H");
                let _ = std::io::stdout().flush();
            }

            match action {
                "right" => {}
                "left" => {
                    println!(
                        "{} {}",
                        style("skip", "33", runtime),
                        file.display_target.display()
                    );
                    return Ok(());
                }
                _ => {
                    bail!(
                        "aborted by user due to conflict in {}",
                        file.display_target.display()
                    );
                }
            }
        }
        backup_file(runtime, &file.target, &file.display_target)?;
    } else if let (false, Some(text)) = (yes, &file.text) {
        crate::utils::print_new_file_content(&file.display_target.to_string_lossy(), text, runtime);
    }
    let write_res = fs::write(&file.target, &file.bytes);
    if let Err(e) = write_res {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            println!("Permission denied for {}.", file.display_target.display());
            if yes
                || confirm(
                    "Attempt to write as root with sudo? [y/N] ",
                    runtime.no_color,
                )?
            {
                write_file_as_root(&file.target, &file.bytes)?;
            } else {
                return Err(e).wrap_err(format!("write {}", file.display_target.display()));
            }
        } else {
            return Err(e).wrap_err(format!("write {}", file.display_target.display()));
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
    let mut command_lines_to_show = Vec::new();
    let dotted = load_dotted(runtime).ok();
    let default_commands = std::collections::HashMap::new();
    let package_commands = dotted
        .as_ref()
        .map_or(&default_commands, |d| &d.config.package_commands);

    for (distro, packages) in &plan.packages {
        let missing: BTreeSet<String> = packages
            .iter()
            .filter(|pkg| !crate::utils::is_package_installed(distro, pkg))
            .cloned()
            .collect();
        if missing.is_empty() {
            continue;
        }
        let command = native_package_command(distro, &missing, package_commands)?;
        command_lines_to_show.push(crate::utils::shell_join(&command));
        println!(
            "native packages ({distro}): {}",
            crate::utils::shell_join(&command)
        );
    }
    let missing_flatpaks: BTreeSet<String> = plan
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
        command_lines_to_show.push(crate::utils::shell_join(&command));
        println!("flatpaks: {}", crate::utils::shell_join(&command));
    }
    for download in &plan.downloads {
        if download.install_path.exists() {
            continue;
        }
        let dl_cmd = match &download.source {
            crate::types::DownloadSource::Url(url) => {
                format!(
                    "curl --fail --location --output {} {}",
                    download.display_path.display(),
                    url
                )
            }
            crate::types::DownloadSource::Zip { url, path } => {
                format!(
                    "curl --fail --location --output archive.zip {} && unzip -p archive.zip {} > {}",
                    url,
                    path,
                    download.display_path.display()
                )
            }
        };
        command_lines_to_show.push(dl_cmd.clone());
        println!("download: {dl_cmd}");
    }

    if !command_lines_to_show.is_empty() {
        println!();
        println!(
            "{}",
            style(
                "COMMANDS PLANNED/EXECUTED FOR PACKAGES/DOWNLOADS:",
                "36;1",
                runtime
            )
        );
        for cmd in &command_lines_to_show {
            println!("  {}", style(cmd, "33", runtime));
        }
    }
    Ok(!command_lines_to_show.is_empty())
}

pub(crate) fn write_env_file(runtime: &Runtime, plan: &Plan) -> Result<()> {
    let dotted = load_dotted(runtime)?;
    let mut out = String::new();
    for (key, value) in &plan.env {
        let _ = writeln!(out, "export {key}={}", shell_escape::escape(value.into()));
    }
    for env_path_str in &dotted.config.env_path {
        let path = runtime.resolve_tilde(env_path_str);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, &out)?;
    }
    Ok(())
}

pub(crate) fn split_artifact_id(id: &str) -> Result<(&str, &str)> {
    if let Some(name) = id.strip_prefix('/') {
        if name.is_empty() || name.contains('/') {
            bail!("artifact id must be /artifact or repo/artifact");
        }
        return Ok(("artifacts", name));
    }
    let (repo, artifact) = id
        .split_once('/')
        .ok_or_else(|| anyhow!("artifact id must be /artifact or repo/artifact"))?;
    if repo.is_empty() || artifact.is_empty() {
        bail!("artifact id must be /artifact or repo/artifact");
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
        other => bail!("unsupported package distro: {other}"),
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
);

pub(crate) fn collect_claims(runtime: &Runtime, artifacts: &[Artifact]) -> Result<Claims> {
    let distro = runtime.distro.clone();
    let arch = normalize_arch(env::consts::ARCH);
    let mut packages: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut flatpaks = BTreeSet::new();
    let mut downloads = BTreeSet::new();
    for artifact in artifacts {
        if let Some(set) = artifact.bin.distro.get(&distro) {
            packages
                .entry(distro.clone())
                .or_default()
                .extend(set.packages.iter().cloned());
        }
        flatpaks.extend(artifact.bin.flatpak.packages.iter().cloned());
        if let Some(download) = artifact.bin.download.get(&arch) {
            downloads.insert(plan_download(runtime, &artifact.id, &arch, download)?.display_path);
        }
    }
    Ok((packages, flatpaks, downloads))
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
            println!("unclaimed native {distro}: {}", installed.join(" "));
        }
    }
    let unclaimed_flatpaks: Vec<_> = target.1.difference(&other.1).cloned().collect();
    let installed_flatpaks: Vec<_> = unclaimed_flatpaks
        .into_iter()
        .filter(|package| crate::utils::is_flatpak_installed(package))
        .collect();
    if !installed_flatpaks.is_empty() {
        println!("unclaimed flatpak: {}", installed_flatpaks.join(" "));
    }
    let unclaimed_downloads: Vec<_> = target.2.difference(&other.2).collect();
    for path in unclaimed_downloads {
        if path.exists() {
            println!(
                "unclaimed download: {}",
                runtime.display_path(path).display()
            );
        }
    }
}

pub(crate) fn restore_one(source: &Path, target: &Path) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, target)?;
    println!("restored {}", target.display());
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
