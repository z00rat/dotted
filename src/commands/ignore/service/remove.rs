/// CLI Command: `ignore service remove [names...]`
///
/// What it does:
/// Removes one or more systemd service unit entries from the global settings ignore list (`ignore.service`).
///
/// Variations:
/// 1. `names` provided: Directly removes the matching service unit entries.
/// 2. `names` empty: Displays an interactive prompt listing all currently ignored service unit entries to pick one for removal.
///
/// Decisions & Logic Branches:
/// - Fails with an error if any specified service unit name is not found in `ignore.service`.
/// - Saves the updated configuration to `settings.toml` upon removal.
use color_eyre::eyre::{Result, bail};

use crate::commands::lib::settings_path;
use crate::types::{Runtime, SettingsFile};
use crate::utils::style;

pub fn run(runtime: &Runtime, names: &[String]) -> Result<()> {
    let settings = settings_path(runtime);
    if !settings.exists() {
        println!("No ignore configuration exists.");
        return Ok(());
    }

    let mut file: SettingsFile = crate::types::read_toml(&settings)?;

    if file.ignore.service.is_empty() {
        println!("Ignored service list is empty.");
        return Ok(());
    }

    if names.is_empty() {
        let selection = if runtime.no_color {
            println!("Select a service ignore entry to remove:");
            for (idx, entry) in file.ignore.service.iter().enumerate() {
                println!("  [{idx}] {entry}");
            }
            loop {
                print!("Enter number to remove: ");
                std::io::Write::flush(&mut std::io::stdout())?;
                let mut line = String::new();
                std::io::stdin().read_line(&mut line)?;
                if let Some(idx) = line
                    .trim()
                    .parse::<usize>()
                    .ok()
                    .filter(|&idx| idx < file.ignore.service.len())
                {
                    break file.ignore.service[idx].clone();
                }
                println!("Invalid selection.");
            }
        } else {
            let mut select = cliclack::select("Select service ignore entry to remove:");
            for (idx, entry) in file.ignore.service.iter().enumerate() {
                select = select.item(idx, entry, "");
            }
            let idx = select.interact()?;
            file.ignore.service[idx].clone()
        };

        file.ignore.service.retain(|x| x != &selection);
        crate::types::write_toml(&settings, &file)?;
        println!(
            "Removed service ignore entry {}",
            style(&selection, "33", runtime)
        );
    } else {
        for target in names {
            let trimmed = target.trim();
            if file.ignore.service.contains(&trimmed.to_string()) {
                file.ignore.service.retain(|x| x != trimmed);
                println!(
                    "Removed service ignore entry {}",
                    style(trimmed, "33", runtime)
                );
            } else {
                bail!("Service ignore entry '{trimmed}' not found in settings.");
            }
        }
        crate::types::write_toml(&settings, &file)?;
    }

    Ok(())
}
