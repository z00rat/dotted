/// CLI Command: `workspace doctor [config|repo|artifact|tool]`
///
/// What it does:
/// Performs a diagnostic system checkup on the dotted workspace configuration and environment.
///
/// Variations:
/// 1. A category provided: Runs only that diagnostic category.
/// 2. No category provided: Runs all diagnostic categories.
///
/// Decisions & Logic Branches:
/// - Finds and parses all workspace configuration TOML files, validating syntax.
/// - Checks the reachability of git remotes for the dotted repository and all configured package repositories.
/// - Checks if the directory exists for all configured package repositories.
/// - Checks if any artifact directories lack an `about.toml` file.
/// - Checks if all artifacts marked as enabled in settings are actually present in the workspace.
/// - Infers needed system CLI tools (e.g., package managers like `pacman`/`dnf`/`apt-get`, `flatpak`, `curl`, `unzip`) from the deployment plan and checks if they are installed.
/// - Exits with a failure if one or more problems are detected; otherwise, succeeds.
use color_eyre::eyre::{Result, WrapErr, bail};
use std::fs;

use crate::commands::lib::{
    artifact_dirs_without_about, check_remote_reachability, check_tool, configured_repos,
    control_toml_paths, load_dotted,
};
use crate::plan::{build_plan, discover_artifacts, load_settings};
use crate::types::Runtime;
use crate::utils::style;

fn log(prefix: &str, text: &str, filter: Option<&str>) {
    let line = format!("{prefix} {text}");
    let matches = if let Some(f) = filter {
        line.to_lowercase().contains(&f.to_lowercase())
    } else {
        true
    };
    if matches {
        println!("{line}");
    }
}

fn check_toml_files(runtime: &Runtime, problems: &mut usize, filter: Option<&str>) -> Result<()> {
    for path in control_toml_paths(runtime)? {
        let path_str = runtime.display_path(&path).display().to_string();
        match fs::read_to_string(&path)
            .wrap_err_with(|| format!("read {}", path.display()))
            .and_then(|content| {
                toml::from_str::<toml::Value>(&content)
                    .wrap_err_with(|| format!("parse {}", path.display()))
            }) {
            Ok(_) => log(
                &style("ok", "32", runtime),
                &format!("toml {path_str}"),
                filter,
            ),
            Err(error) => {
                *problems += 1;
                log(
                    &style("bad", "31", runtime),
                    &format!("toml {path_str}: {error:#}"),
                    filter,
                );
            }
        }
    }
    Ok(())
}

fn check_repos(runtime: &Runtime, problems: &mut usize, filter: Option<&str>) -> Result<()> {
    let dotted = load_dotted(runtime)?;
    for repo in configured_repos(&dotted) {
        let path = runtime.dotted_dir.join(&repo.name);
        if path.is_dir() {
            log(
                &style("ok", "32", runtime),
                &format!("repo {} {}", repo.name, path.display()),
                filter,
            );
            check_remote_reachability(
                runtime,
                &path,
                &format!("repo {}", repo.name),
                problems,
                filter,
            );
        } else {
            *problems += 1;
            log(
                &style("missing", "31", runtime),
                &format!("repo {} {}", repo.name, path.display()),
                filter,
            );
        }
    }
    Ok(())
}

fn check_plan_tools(runtime: &Runtime, problems: &mut usize, filter: Option<&str>) {
    let plan = build_plan(runtime, None).ok();
    if let Some(plan) = plan {
        for distro in plan.packages.keys() {
            let tool = match distro.as_str() {
                "archlinux" => "pacman",
                "fedora" => "dnf",
                "ubuntu" => "apt-get",
                _ => "",
            };
            if !tool.is_empty() {
                check_tool(runtime, tool, problems, filter);
            }
        }
        if !plan.flatpaks.is_empty() {
            check_tool(runtime, "flatpak", problems, filter);
        }
        if !plan.downloads.is_empty() {
            check_tool(runtime, "curl", problems, filter);
            if plan
                .downloads
                .iter()
                .any(|download| matches!(download.source, crate::types::DownloadSource::Zip { .. }))
            {
                check_tool(runtime, "unzip", problems, filter);
            }
        }
    }
}

pub fn run(runtime: &Runtime, filter: Option<&str>) -> Result<()> {
    let enabled = |name: &str| filter.is_none_or(|f| f == name);
    let mut problems = 0usize;

    log(
        "",
        &format!(
            "checking {}",
            runtime.display_path(&runtime.dotted_dir).display()
        ),
        None,
    );

    if enabled("config") {
        check_toml_files(runtime, &mut problems, None)?;
    }

    if enabled("repo") {
        check_remote_reachability(
            runtime,
            &runtime.dotted_dir,
            "dotted repo",
            &mut problems,
            None,
        );
    }

    if enabled("repo") {
        check_repos(runtime, &mut problems, None)?;
    }

    let artifacts = discover_artifacts(runtime)?;
    if enabled("artifact") {
        for (repo, name, dir) in artifact_dirs_without_about(runtime)? {
            problems += 1;
            log(
                "",
                &format!(
                    "artifact folder without [about] entry: {repo}/{name} {}",
                    dir.display()
                ),
                None,
            );
        }
        let settings = load_settings(runtime)?;
        for id in &settings.enable {
            if artifacts.contains_key(id) {
                log(&style("ok", "32", runtime), &format!("enabled {id}"), None);
            } else {
                problems += 1;
                log(
                    &style("missing", "31", runtime),
                    &format!("enabled artifact {id}"),
                    None,
                );
            }
        }
    }

    if enabled("tool") {
        check_plan_tools(runtime, &mut problems, None);
    }

    if problems == 0 {
        log("", &style("Doctor: ok", "32", runtime), None);
        Ok(())
    } else {
        bail!("doctor found {problems} problem(s)")
    }
}
