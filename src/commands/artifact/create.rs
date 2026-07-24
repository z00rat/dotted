/// CLI Command: `artifact create <artifact_name>`
///
/// What it does:
/// Creates a new empty artifact under `[artifacts]/` (or under `repo/name` if repository is specified).
///
/// Variations:
/// 1. `artifact_name` provided as a single word (e.g. `meow`): Creates inside `[artifacts]/meow`.
/// 2. `artifact_name` provided as `repo/meow`: Creates inside `[repo]/meow`.
///
/// Decisions & Logic Branches:
/// - Creates the destination directory inside the corresponding repository folder.
/// - Ensures that `[about].toml` in that repository contains the new artifact entry.
use color_eyre::eyre::Result;
use std::fs;

use crate::commands::lib::{ensure_about_entry, repository_path};
use crate::types::Runtime;
use crate::utils::style;

pub fn run(runtime: &Runtime, artifact_name: &str) -> Result<()> {
    crate::utils::print_banner("CREATING NEW ARTIFACT", runtime);
    let (repo, name) = if let Some((r, a)) = artifact_name.split_once('/') {
        if r.is_empty() || a.is_empty() || a.contains('/') {
            color_eyre::eyre::bail!("invalid artifact name format: expected 'name' or 'repo/name'");
        }
        (r, a)
    } else if artifact_name.starts_with('/') {
        let name = artifact_name.trim_start_matches('/');
        if name.is_empty() || name.contains('/') {
            color_eyre::eyre::bail!("invalid artifact name format");
        }
        ("artifacts", name)
    } else {
        ("artifacts", artifact_name)
    };

    if name == "." || name == ".." || name.contains('\\') {
        color_eyre::eyre::bail!("invalid artifact name");
    }

    let artifact_dir = repository_path(runtime, repo).join(name);
    fs::create_dir_all(&artifact_dir)?;
    ensure_about_entry(runtime, repo, name)?;
    let display_id = if repo == "artifacts" {
        format!("/{name}")
    } else {
        format!("{repo}/{name}")
    };
    println!("created {}", style(&display_id, "32;1", runtime));
    Ok(())
}
