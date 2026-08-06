/// CLI Command: `ignore file add [paths...]`
///
/// What it does:
/// Appends one or more file or directory paths to the user's global settings ignore list (`ignore.file` or `ignore.folder`).
///
/// Variations:
/// 1. `paths` provided: Converts each path to absolute, resolves home-relative paths with `~/`, and appends to settings.
/// 2. `paths` empty: Launches an interactive terminal-based file browser to let the user select a file or folder to ignore.
///
/// Decisions & Logic Branches:
/// - Resolves home-relative paths to `~/` format for portability.
/// - Determines whether each target path is a directory (using filesystem checks or missing file extension).
/// - Inserts into `ignore.folder` if directory, or `ignore.file` if file, avoiding duplicates.
/// - Sorts the ignore entries and writes the updated `settings.toml`.
use color_eyre::eyre::Result;
use std::path::PathBuf;

use crate::commands::lib::settings_path;
use crate::types::{Runtime, SettingsFile};
use crate::utils::style;

pub fn run(runtime: &Runtime, paths: &[PathBuf]) -> Result<()> {
    let targets = if paths.is_empty() {
        vec![crate::commands::adopt::file::select_path_for_ignore()?]
    } else {
        paths.to_vec()
    };
    let settings = settings_path(runtime);
    let mut file: SettingsFile = if settings.exists() {
        crate::types::read_toml(&settings)?
    } else {
        SettingsFile::default()
    };

    for selected in targets {
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
            println!("Ignored folder {}", style(&value, "32", runtime));
        } else {
            if !file.ignore.file.contains(&value) {
                file.ignore.file.push(value.clone());
            }
            println!("Ignored file {}", style(&value, "32", runtime));
        }
    }
    file.ignore.file.sort();
    file.ignore.folder.sort();
    crate::types::write_toml(&settings, &file)?;
    Ok(())
}
