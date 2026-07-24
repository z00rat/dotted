/// CLI Commands: `files ignore add <path>` and `files ignore remove [path]`
///
/// What they do:
/// Manages directory and file ignore rules by writing them to the current user's settings file.
///
/// Variations:
/// 1. `files ignore add <path>`:
///    - Takes a file/folder path and appends it to the settings' ignore lists.
/// 2. `files ignore remove [path]`:
///    - If `path` is provided: Directly removes that ignore entry.
///    - If `path` is not provided: Runs an interactive selection menu displaying all current ignore entries, allowing the user to select one to delete.
///
/// Decisions & Logic Branches:
/// - In `add`:
///   - Converts the path to absolute, replacing home-relative paths with `~/`.
///   - Infers if the path is a directory (using filesystem checks or looking for the absence of a file extension).
///   - Appends it to the corresponding list (`ignore.folder` or `ignore.file`), sorts the list, and saves the settings file.
/// - In `remove`:
///   - If matching a provided path, searches and removes the entry, failing if not found.
///   - If interactive, shows a list of entries, detects whether the selected entry was a file or directory based on selection label prefix, removes it, and saves.
use color_eyre::eyre::{Result, bail};
use std::path::Path;

use crate::commands::lib::settings_path;
use crate::types::{Runtime, SettingsFile};
use crate::utils::style;

pub fn add(runtime: &Runtime, path: Option<&Path>) -> Result<()> {
    let selected = match path {
        Some(path) => path.to_path_buf(),
        None => crate::commands::adopt::file::select_path_for_ignore()?,
    };
    let settings = settings_path(runtime);
    let mut file: SettingsFile = if settings.exists() {
        crate::types::read_toml(&settings)?
    } else {
        SettingsFile::default()
    };

    let abs_path = if selected.is_absolute() {
        selected.clone()
    } else {
        std::env::current_dir()?.join(&selected)
    };

    let value = if let Ok(rest) = abs_path.strip_prefix(&runtime.home_dir) {
        format!("~/{}", rest.to_string_lossy())
    } else {
        abs_path.to_string_lossy().to_string()
    };

    let is_dir = if abs_path.exists() {
        abs_path.is_dir()
    } else {
        selected.extension().is_none()
    };

    if is_dir {
        if !file.ignore.folder.contains(&value) {
            file.ignore.folder.push(value.clone());
        }
    } else if !file.ignore.file.contains(&value) {
        file.ignore.file.push(value.clone());
    }
    file.ignore.file.sort();
    file.ignore.folder.sort();
    crate::types::write_toml(&settings, &file)?;
    println!("Ignored {}", style(&value, "32", runtime));
    Ok(())
}

pub fn remove(runtime: &Runtime, path: Option<&Path>) -> Result<()> {
    let settings = settings_path(runtime);
    if !settings.exists() {
        println!("No ignore configuration exists.");
        return Ok(());
    }

    let mut file: SettingsFile = crate::types::read_toml(&settings)?;

    let target_value = if let Some(p) = path {
        let abs_path = if p.is_absolute() {
            p.to_path_buf()
        } else {
            std::env::current_dir()?.join(p)
        };
        let value = if let Ok(rest) = abs_path.strip_prefix(&runtime.home_dir) {
            format!("~/{}", rest.to_string_lossy())
        } else {
            abs_path.to_string_lossy().to_string()
        };
        Some(value)
    } else {
        None
    };

    if let Some(ref val) = target_value {
        let mut removed = false;
        if file.ignore.file.contains(val) {
            file.ignore.file.retain(|x| x != val);
            removed = true;
        }
        if file.ignore.folder.contains(val) {
            file.ignore.folder.retain(|x| x != val);
            removed = true;
        }

        if removed {
            crate::types::write_toml(&settings, &file)?;
            println!("Removed ignore entry {}", style(val, "33", runtime));
        } else {
            bail!("Ignore entry '{val}' not found in settings.");
        }
    } else {
        // Interactive CLI prompt
        let all_entries: Vec<String> = file
            .ignore
            .folder
            .iter()
            .map(|f| format!("(dir)  {f}"))
            .chain(file.ignore.file.iter().map(|f| format!("(file) {f}")))
            .collect();

        if all_entries.is_empty() {
            println!("Ignore list is empty.");
            return Ok(());
        }

        let selection = if runtime.no_color {
            println!("Select an entry to remove:");
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
            let mut select = cliclack::select("Select ignore entry to remove:");
            for entry in &all_entries {
                select = select.item(entry.clone(), entry, "");
            }
            select.interact()?
        };

        let is_dir = selection.starts_with("(dir)  ");
        let val = selection[7..].to_string();

        if is_dir {
            file.ignore.folder.retain(|x| x != &val);
        } else {
            file.ignore.file.retain(|x| x != &val);
        }

        crate::types::write_toml(&settings, &file)?;
        println!("Removed ignore entry {}", style(&val, "33", runtime));
    }

    Ok(())
}
