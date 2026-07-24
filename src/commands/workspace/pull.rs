/// CLI Command: `workspace pull`
///
/// What it does:
/// Syncs/pulls changes for the dotted repository itself and any configured package/dotfile repositories from their remotes.
///
/// Variations:
/// None (standard execution).
///
/// Decisions & Logic Branches:
/// - Exits early if no Git remote is configured for either the dotted repo or any repository.
/// - Runs without confirmation.
/// - For the dotted repository (if it is a git repo): Pulls changes via `git pull --ff-only`.
/// - For each package repository configured in `dotted.toml`:
///   - If it has a remote URL:
///     - Clones the repository if the local folder doesn't exist.
///     - Pulls changes via `git pull --ff-only` if the local folder exists.
///     - Checks out the configured branch, tag, or revision if specified.
use color_eyre::eyre::Result;

use crate::commands::lib::{checkout_repo, configured_repos, load_dotted};
use crate::types::Runtime;
use crate::utils::run_git;

pub fn run(runtime: &Runtime) -> Result<()> {
    let dotted = load_dotted(runtime)?;

    let meta_git = runtime.dotted_dir.join(".git").exists();
    let has_remote = meta_git || !configured_repos(&dotted).is_empty();
    if !has_remote {
        println!("No remote git repositories configured to sync.");
        return Ok(());
    }

    println!("Plan to sync the following repositories:");
    if meta_git {
        println!(
            "  - dotted repo (pull) -> {}",
            runtime.display_path(&runtime.dotted_dir).display()
        );
    } else {
        println!(
            "  - dotted repo (local, not git) -> {}",
            runtime.display_path(&runtime.dotted_dir).display()
        );
    }
    for repo in configured_repos(&dotted) {
        let path = runtime.dotted_dir.join(&repo.name);
        let url = &repo.url;
        if path.exists() {
            println!(
                "  - repo {} (pull from {}) -> {}",
                repo.name,
                url,
                runtime.display_path(&path).display()
            );
        } else {
            println!(
                "  - repo {} (clone from {}) -> {}",
                repo.name,
                url,
                runtime.display_path(&path).display()
            );
        }
    }
    println!();

    if meta_git {
        let _ = run_git(&runtime.dotted_dir, ["pull", "--ff-only"]);
    }
    for repo in configured_repos(&dotted) {
        let path = runtime.dotted_dir.join(&repo.name);
        let url = &repo.url;
        if path.exists() {
            run_git(&path, ["pull", "--ff-only"])?;
        } else {
            run_git(
                &runtime.dotted_dir,
                ["clone", url.as_str(), repo.name.as_str()],
            )?;
        }
        checkout_repo(&path, &repo)?;
    }
    Ok(())
}
