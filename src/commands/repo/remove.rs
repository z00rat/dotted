/// CLI Command: `repo remove <name>`
///
/// What it does:
/// Removes a repository's configuration from `dotted.toml`, disables all of its artifacts in settings, and warns the user about the disk directory.
///
/// Variations:
/// None (requires repository name).
///
/// Decisions & Logic Branches:
/// - Fails if the repository is not found in the `[[repo]]` entries of `dotted.toml`.
/// - Asks for confirmation before removing.
/// - Saves changes to `dotted.toml`.
/// - Disables all enabled artifacts in settings that belong to the removed repository (matching artifact IDs prefixed with `name/`).
/// - Prints a warning that the actual repository directory remains on disk (does not delete files automatically).
use color_eyre::eyre::{Result, bail};
use std::collections::BTreeSet;

use crate::commands::lib::{load_dotted, settings_path};
use crate::types::{Runtime, SettingsFile};
use crate::utils::confirm;

pub fn run(runtime: &Runtime, name: &str) -> Result<()> {
    crate::utils::print_banner("REMOVING REPOSITORY CONFIGURATION", runtime);
    let mut dotted = load_dotted(runtime)?;

    let is_other = dotted.repos.iter().any(|r| r.name == name);

    if !is_other {
        bail!("Repository '{name}' not found in configuration.");
    }

    if !confirm(
        &format!("Remove repository '{name}' from configuration?"),
        runtime.no_color,
    )? {
        return Ok(());
    }
    dotted.repos.retain(|r| r.name != name);

    crate::types::write_toml(&runtime.dotted_path(), &dotted)?;
    println!("Removed repo {}", crate::utils::style(name, "33", runtime));

    // Disable all artifacts under this repository
    let path = settings_path(runtime);
    if path.exists() {
        let mut file: SettingsFile = crate::types::read_toml(&path)?;
        let mut enable: BTreeSet<String> = file.artifacts.enable.into_iter().collect();
        let mut disable: BTreeSet<String> = file.artifacts.disable.into_iter().collect();

        let prefix = format!("{name}/");
        let mut disabled_any = false;
        let mut to_disable = Vec::new();
        for id in &enable {
            if id.starts_with(&prefix) {
                to_disable.push(id.clone());
            }
        }
        for id in to_disable {
            enable.remove(&id);
            disable.insert(id);
            disabled_any = true;
        }

        if disabled_any {
            file.artifacts.enable = enable.into_iter().collect();
            file.artifacts.disable = disable.into_iter().collect();
            crate::types::write_toml(&path, &file)?;
            println!(
                "Disabled artifacts for repo {}",
                crate::utils::style(name, "33", runtime)
            );
        }
    }

    // Warn about disk directory
    let repo_dir = runtime.dotted_dir.join(name);
    if repo_dir.exists() {
        eprintln!(
            "Warning: The repository directory remains on disk at '{}'.",
            runtime.display_path(&repo_dir).display()
        );
    }

    Ok(())
}
