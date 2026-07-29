/// CLI Command: `ignore service add [names...]`
///
/// What it does:
/// Appends one or more systemd service unit names (e.g., `syncthing.service` or `user:syncthing.service`) to the global settings ignore list (`ignore.service`).
///
/// Variations:
/// 1. `names` provided: Directly appends the specified service unit names to `ignore.service`.
/// 2. `names` empty: Prompts the user to enter a service unit name interactively.
///
/// Decisions & Logic Branches:
/// - Trims input string and verifies non-emptiness.
/// - Prevents duplicate entries in `ignore.service`.
/// - Sorts the list alphabetically and updates `settings.toml`.
use color_eyre::eyre::Result;

use crate::commands::lib::settings_path;
use crate::types::{Runtime, SettingsFile};
use crate::utils::style;

pub fn run(runtime: &Runtime, names: &[String]) -> Result<()> {
    let targets = if names.is_empty() {
        print!("Enter service unit to ignore (e.g. syncthing.service or user:syncthing.service): ");
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let trimmed = input.trim().to_string();
        if trimmed.is_empty() {
            println!("No service specified.");
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
        if file.ignore.service.contains(&trimmed.to_string()) {
            println!(
                "Service '{}' is already in ignore list.",
                style(trimmed, "33", runtime)
            );
        } else {
            file.ignore.service.push(trimmed.to_string());
            println!("Ignored service {}", style(trimmed, "32", runtime));
        }
    }
    file.ignore.service.sort();
    crate::types::write_toml(&settings, &file)?;

    Ok(())
}
