/// CLI Command: `ignore package add [names...]`
///
/// What it does:
/// Appends one or more system package names (e.g., native package or Flatpak) to the global settings ignore list (`ignore.package`).
///
/// Variations:
/// 1. `names` provided: Directly adds the specified package names to `ignore.package`.
/// 2. `names` empty: Prompts the user to enter a package name interactively.
///
/// Decisions & Logic Branches:
/// - Trims input string and verifies non-emptiness.
/// - Prevents duplicate entries in `ignore.package`.
/// - Sorts the list alphabetically and updates `settings.toml`.
use color_eyre::eyre::Result;

use crate::commands::lib::settings_path;
use crate::types::{Runtime, SettingsFile};
use crate::utils::style;

pub fn run(runtime: &Runtime, names: &[String]) -> Result<()> {
    let targets = if names.is_empty() {
        print!("Enter package name to ignore: ");
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let trimmed = input.trim().to_string();
        if trimmed.is_empty() {
            println!("No package specified.");
            return Ok(());
        }
        vec![trimmed]
    } else {
        names.to_vec()
    };

    let settings = settings_path(runtime);
    let mut file: SettingsFile = if settings.exists() {
        crate::types::read_toml(&settings)?
    } else {
        SettingsFile::default()
    };

    for name_str in targets {
        let trimmed = name_str.trim();
        if trimmed.is_empty() {
            continue;
        }
        if file.ignore.package.contains(&trimmed.to_string()) {
            println!(
                "Package '{}' is already in ignore list.",
                style(trimmed, "33", runtime)
            );
        } else {
            file.ignore.package.push(trimmed.to_string());
            println!("Ignored package {}", style(trimmed, "32", runtime));
        }
    }
    file.ignore.package.sort();
    crate::types::write_toml(&settings, &file)?;

    Ok(())
}
