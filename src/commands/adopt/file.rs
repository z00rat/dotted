/// CLI Command: `adopt file <artifact_id> [paths...]`
///
/// What it does:
/// Copies selected system files or directories into an artifact in any configured repository.
///
/// Variations:
/// 1. `paths` provided: Directly adopts all specified files/directories.
/// 2. `paths` not provided (interactive): Runs an interactive terminal-based file browser to let the user navigate, pick a file, or type a path.
/// 3. `paths` not provided (non-interactive): Fails with an error.
///
/// Decisions & Logic Branches:
/// - Computes workspace-relative source paths using `artifact_relative_from_system_path`.
/// - Displays file diffs and prompts for conflict resolution (overwrite, keep/skip, or abort) if a target file already exists in the artifact directory.
/// - Copies files into place and records the artifact in `[about].toml`; adoption never enables it.
use color_eyre::eyre::{Result, bail};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

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

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdoptConflictAction {
    Overwrite,
    Keep,
    Abort,
}

fn prompt_adopt_conflict_no_color(destination: &Path) -> Result<AdoptConflictAction> {
    loop {
        print!(
            "Conflict in {}. [r]ight (overwrite artifact), [l]eft (keep current artifact), [a]bort? ",
            destination.display()
        );
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        match line.trim().to_ascii_lowercase().as_str() {
            "r" | "right" | "y" | "yes" => return Ok(AdoptConflictAction::Overwrite),
            "l" | "left" | "n" | "no" => return Ok(AdoptConflictAction::Keep),
            "a" | "abort" => return Ok(AdoptConflictAction::Abort),
            _ => println!("invalid choice. Please enter 'r', 'l', or 'a'."),
        }
    }
}

fn prompt_adopt_conflict(destination: &Path, runtime: &Runtime) -> Result<AdoptConflictAction> {
    if runtime.no_color {
        prompt_adopt_conflict_no_color(destination)
    } else {
        let action = cliclack::select(format!("Conflict while adopting {}", destination.display()))
            .item(
                AdoptConflictAction::Overwrite,
                "Overwrite artifact file with source",
                "",
            )
            .item(
                AdoptConflictAction::Keep,
                "Keep current artifact file (skip)",
                "",
            )
            .item(AdoptConflictAction::Abort, "Abort adoption", "")
            .interact()
            .map_err(|e| color_eyre::eyre::Report::msg(e.to_string()))?;

        print!("\x1B[2J\x1B[H");
        let _ = std::io::stdout().flush();

        Ok(action)
    }
}

fn adopt_single_path(
    runtime: &Runtime,
    artifact_id: &str,
    src: &Path,
    destination: &Path,
) -> Result<()> {
    if src.is_dir() {
        copy_dir_all(src, destination)?;
    } else {
        if destination.exists() {
            let current_dst_bytes = fs::read(destination)?;
            let src_bytes = fs::read(src)?;
            if current_dst_bytes == src_bytes {
                println!(
                    "Adopted {} into {} ({})",
                    style(&src.to_string_lossy(), "32", runtime),
                    style(artifact_id, "36;1", runtime),
                    style("unchanged", "32", runtime)
                );
                return Ok(());
            }

            let display_target = destination.to_path_buf();
            let planned_file = crate::types::PlannedFile {
                artifact_id: artifact_id.to_string(),
                source: src.to_path_buf(),
                target: destination.to_path_buf(),
                display_target,
                bytes: src_bytes,
                text: String::from_utf8(fs::read(src)?).ok(),
            };

            crate::utils::show_file_diff(&planned_file, &current_dst_bytes, runtime);

            let action = prompt_adopt_conflict(destination, runtime)?;
            match action {
                AdoptConflictAction::Overwrite => {}
                AdoptConflictAction::Keep => {
                    println!("{} {}", style("skip", "33", runtime), destination.display());
                    return Ok(());
                }
                AdoptConflictAction::Abort => {
                    bail!(
                        "aborted by user due to conflict in {}",
                        destination.display()
                    );
                }
            }
        }

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, destination)?;
    }
    crate::utils::ensure_user_writable(destination)?;
    crate::utils::chown_path_tree_if_root(runtime, destination)?;
    println!(
        "Adopted {} into {}",
        style(&src.to_string_lossy(), "32", runtime),
        style(artifact_id, "36;1", runtime)
    );
    Ok(())
}

pub fn run(runtime: &Runtime, artifact_id: &str, paths: Vec<PathBuf>) -> Result<()> {
    let (repo, artifact) = split_artifact_id(artifact_id)?;
    let target_paths = if !paths.is_empty() {
        paths
    } else if !runtime.no_color {
        vec![select_path_for_ignore()?]
    } else {
        bail!("adopt requires at least one path when running non-interactively");
    };

    let repo_dir = repository_path(runtime, repo);
    let artifact_dir = repo_dir.join(artifact);

    let mut planned = Vec::new();
    for src in &target_paths {
        let relative = artifact_relative_from_system_path(runtime, src);
        let destination = artifact_dir.join(&relative);
        planned.push((src, destination));
    }

    for (src, destination) in &planned {
        adopt_single_path(runtime, artifact_id, src, destination)?;
    }

    ensure_about_entry(runtime, repo, artifact)?;
    crate::utils::chown_path_tree_if_root(runtime, &repo_dir)?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_adopt_multiple_files() {
        let temp = TempDir::new().unwrap();
        let home_dir = temp.path().join("home");
        fs::create_dir_all(&home_dir).unwrap();
        let home_dir = home_dir.canonicalize().unwrap();

        let runtime = Runtime {
            dotted_dir: temp.path().join("dotted"),
            home_dir: home_dir.clone(),
            root_dir: temp.path().join("root"),
            user: "user".to_string(),
            device: "device".to_string(),
            distro: "archlinux".to_string(),
            no_color: true,
            dotted: crate::types::DottedFile::default(),
        };

        let artifact_dir = runtime.dotted_dir.join("[artifacts]").join("myart");
        fs::create_dir_all(&artifact_dir).unwrap();

        let file1 = home_dir.join(".config/app/file1.txt");
        let file2 = home_dir.join(".config/app/file2.txt");
        fs::create_dir_all(file1.parent().unwrap()).unwrap();
        fs::write(&file1, "content1").unwrap();
        fs::write(&file2, "content2").unwrap();

        run(&runtime, "/myart", vec![file1, file2]).unwrap();

        assert!(artifact_dir.join("home/.config/app/file1.txt").exists());
        assert!(artifact_dir.join("home/.config/app/file2.txt").exists());
    }
}
