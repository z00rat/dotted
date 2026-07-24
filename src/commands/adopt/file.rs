/// CLI Command: `adopt file <artifact_id> [path]`
///
/// What it does:
/// Copies a selected system file or directory into an artifact in any configured repository.
///
/// Variations:
/// 1. `path` provided: Directly adopts the specified file.
/// 2. `path` not provided (interactive): Runs an interactive terminal-based file browser to let the user navigate, pick a file, or type a path.
/// 3. `path` not provided (non-interactive): Fails with an error.
///
/// Decisions & Logic Branches:
/// - Computes the workspace-relative source path using `artifact_relative_from_system_path`.
/// - Fails if the file already exists at the computed target path in the artifact directory.
/// - Copies the file and records the artifact in `[about].toml`; adoption never enables it.
use color_eyre::eyre::{Result, bail};
use std::fs;
use std::path::PathBuf;

use crate::commands::lib::{
    artifact_relative_from_system_path, ensure_about_entry, repository_path, split_artifact_id,
};
use crate::types::Runtime;
use crate::utils::style;

#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuChoice {
    Index(usize),
    Up,
    Custom,
}

pub(crate) fn select_path_for_ignore() -> Result<PathBuf> {
    let mut current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        let current_dir_canonical = current_dir
            .canonicalize()
            .unwrap_or_else(|_| current_dir.clone());
        println!("Current directory: {}", current_dir_canonical.display());

        let mut entries = Vec::new();
        if let Ok(read_dir) = fs::read_dir(&current_dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                let display_name = if path.is_dir() {
                    format!("{name}/")
                } else {
                    name
                };
                entries.push((path, display_name));
            }
        }
        // Sort entries: directories first, then files
        entries.sort_by(|a, b| {
            let a_is_dir = a.0.is_dir();
            let b_is_dir = b.0.is_dir();
            if a_is_dir && !b_is_dir {
                std::cmp::Ordering::Less
            } else if !a_is_dir && b_is_dir {
                std::cmp::Ordering::Greater
            } else {
                a.1.to_lowercase().cmp(&b.1.to_lowercase())
            }
        });

        let mut select = cliclack::select("Select a file or directory to adopt:");
        for (i, (_, display)) in entries.iter().enumerate() {
            select = select.item(MenuChoice::Index(i), display, "");
        }
        if current_dir.parent().is_some() {
            select = select.item(MenuChoice::Up, "../ (parent directory)", "");
        }
        select = select.item(MenuChoice::Custom, "[Type custom path]", "");

        let choice = select
            .interact()
            .map_err(|e| color_eyre::eyre::Report::msg(e.to_string()))?;
        match choice {
            MenuChoice::Custom => {
                let input_str = cliclack::input("Enter path to adopt:")
                    .placeholder("/path/to/file")
                    .validate(|input: &String| {
                        if input.trim().is_empty() {
                            Err("Path cannot be empty")
                        } else if !std::path::Path::new(input.trim()).exists() {
                            Err("Path does not exist")
                        } else {
                            Ok(())
                        }
                    })
                    .interact::<String>()
                    .map_err(|e| color_eyre::eyre::Report::msg(e.to_string()))?;
                return Ok(PathBuf::from(input_str.trim()));
            }
            MenuChoice::Up => {
                if let Some(parent) = current_dir.parent() {
                    current_dir = parent.to_path_buf();
                }
            }
            MenuChoice::Index(idx) => {
                let chosen_path = &entries[idx].0;
                if chosen_path.is_dir() {
                    current_dir.clone_from(chosen_path);
                } else {
                    return Ok(chosen_path.clone());
                }
            }
        }
    }
}

pub fn run(runtime: &Runtime, artifact_id: &str, path: Option<PathBuf>) -> Result<()> {
    crate::utils::print_banner("ADOPTING ARTIFACT", runtime);
    let (repo, artifact) = split_artifact_id(artifact_id)?;
    let path = if let Some(p) = path {
        p
    } else if !runtime.no_color {
        select_path_for_ignore()?
    } else {
        bail!("adopt requires a path when running non-interactively");
    };

    let relative = artifact_relative_from_system_path(runtime, &path);
    let destination = repository_path(runtime, repo).join(artifact).join(relative);
    if destination.exists() {
        bail!("artifact file already exists: {}", destination.display());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(&path, &destination)?;
    ensure_about_entry(runtime, repo, artifact)?;
    println!(
        "adopted {} into {}",
        style(&path.to_string_lossy(), "32", runtime),
        style(artifact_id, "36;1", runtime)
    );
    Ok(())
}
