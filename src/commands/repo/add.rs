/// CLI Command: `repo add <name> <git_url>`
///
/// What it does:
/// Adds a required `[[repo]]` Git repository entry and clones or pulls it immediately.
///
/// Variations:
/// None (requires repository name and remote Git URL).
///
/// Decisions & Logic Branches:
/// - Fails if a repository with the specified name is already configured.
/// - Persists the valid repository URL before attempting the clone.
/// - A failed clone leaves the configuration in place so `workspace pull` can retry.
use color_eyre::eyre::{Result, bail};

use crate::commands::lib::{configured_repos, load_dotted};
use crate::types::{RepoConfig, Runtime};

pub fn run(runtime: &Runtime, name: &str, git_url: &str) -> Result<()> {
    crate::utils::print_banner("ADDING REPOSITORY CONFIGURATION", runtime);
    let mut dotted = load_dotted(runtime)?;
    if configured_repos(&dotted)
        .iter()
        .any(|repo| repo.name == name)
    {
        bail!("repo already exists: {name}");
    }
    dotted.repos.push(RepoConfig {
        name: name.to_string(),
        url: git_url.to_string(),
        branch: None,
        tag: None,
        revision: None,
    });
    dotted
        .repos
        .sort_by(|left, right| left.name.cmp(&right.name));
    crate::types::write_toml(&runtime.dotted_path(), &dotted)?;
    let repo_path = runtime.dotted_dir.join(name);
    if repo_path.exists() {
        crate::utils::run_git(&repo_path, ["pull", "--ff-only"])?;
    } else {
        crate::utils::run_git(&runtime.dotted_dir, ["clone", git_url, name])?;
    }
    println!("added repo {name}");
    Ok(())
}
