/// CLI Command: `ignore file remove [paths...]`
///
/// What it does:
/// Removes one or more file or directory path entries from the user's global settings ignore lists.
///
/// Variations:
/// 1. `paths` provided: Directly searches and removes matching file or folder entries from settings.
/// 2. `paths` empty: Displays an interactive selection menu containing all current file and directory ignore rules.
///
/// Decisions & Logic Branches:
/// - Strips home prefix and normalizes paths to match stored `~/` format.
/// - If interactive, formats choices with `(dir)` and `(file)` indicators.
/// - Saves the updated configuration to `settings.toml` upon removal.
use color_eyre::eyre::{Result, bail};
use std::path::PathBuf;

use crate::commands::lib::settings_path;
use crate::types::{Runtime, SettingsFile};
use crate::utils::style;

pub fn run(runtime: &Runtime, paths: &[PathBuf]) -> Result<()> {
    let settings = settings_path(runtime);
    if !settings.exists() {
        println!("No ignore configuration exists.");
        return Ok(());
    }

    let mut file: SettingsFile = crate::types::read_toml(&settings)?;

    if paths.is_empty() {
        let all_entries: Vec<String> = file
            .ignore
            .folder
            .iter()
            .map(|f| format!("(dir)  {f}"))
            .chain(file.ignore.file.iter().map(|f| format!("(file) {f}")))
            .collect();

        if all_entries.is_empty() {
            println!("The ignored files or folders list is empty.");
            return Ok(());
        }

        let selection = if runtime.no_color {
            println!("Select a file or folder ignore entry to remove:");
            for (idx, entry) in all_entries.iter().enumerate() {
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
                    .filter(|&idx| idx < all_entries.len())
                {
                    break all_entries[idx].clone();
                }
                println!("Invalid selection.");
            }
        } else {
            let mut select = cliclack::select("Select a file or folder ignore entry to remove:");
            for (idx, entry) in all_entries.iter().enumerate() {
                select = select.item(idx, entry, "");
            }
            let idx = select.interact()?;
            all_entries[idx].clone()
        };

        let raw = selection
            .strip_prefix("(dir)  ")
            .or_else(|| selection.strip_prefix("(file) "))
            .unwrap_or(&selection);

        file.ignore.folder.retain(|x| x != raw);
        file.ignore.file.retain(|x| x != raw);
        crate::types::write_toml(&settings, &file)?;
        println!("Removed ignore entry {}", style(raw, "33", runtime));
    } else {
        for p in paths {
            let abs_path = if p.is_absolute() {
                p.clone()
            } else {
                std::env::current_dir()?.join(p)
            };
            let val = if let Ok(rest) = abs_path.strip_prefix(&runtime.home_dir) {
                format!("~/{}", rest.to_string_lossy())
            } else {
                abs_path.to_string_lossy().to_string()
            };

            let mut removed = false;
            if file.ignore.file.contains(&val) {
                file.ignore.file.retain(|x| x != &val);
                removed = true;
            }
            if file.ignore.folder.contains(&val) {
                file.ignore.folder.retain(|x| x != &val);
                removed = true;
            }

            if removed {
                println!("Removed ignore entry {}", style(&val, "33", runtime));
            } else {
                bail!("Ignore entry '{val}' not found in settings.");
            }
        }
        crate::types::write_toml(&settings, &file)?;
    }

    Ok(())
}
