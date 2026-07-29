/// CLI Command: `workspace format`
///
/// What it does:
/// Formats all `dotted` configuration TOML files (`[dotted].toml`, `[local].toml`, `[settings]/**/*.toml`, `[about].toml`, and `[bin].toml`).
///
/// Variations:
/// 1. Run without arguments: Formats `dotted` configuration TOML files in the active workspace.
///
/// Decisions & Logic Branches:
/// - Only targets `dotted` configuration files (`[dotted].toml`, `[local].toml`, `[settings]/**/*.toml`, `[about].toml`, `[bin].toml`).
/// - Skips user dotfiles and managed system template files stored inside artifacts (e.g. `home/.config/*.toml`).
/// - Reads each control TOML file, parses it into `toml::Value`, and formats it using `toml::to_string_pretty`.
/// - Writes back formatted content if it differs from disk and prints a green status message.
use color_eyre::eyre::{Context, Result};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use crate::types::Runtime;
use crate::utils::style;

fn is_dotted_config_toml(path: &Path, dotted_dir: &Path) -> bool {
    if matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some("[dotted].toml" | "[local].toml" | "[about].toml" | "[bin].toml")
    ) {
        return true;
    }
    if let Ok(rel) = path.strip_prefix(dotted_dir) {
        return rel.starts_with("[settings]") && path.extension().is_some_and(|ext| ext == "toml");
    }
    false
}

pub fn run(runtime: &Runtime) -> Result<()> {
    let mut formatted_count = 0;
    for entry in WalkDir::new(&runtime.dotted_dir)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            name != ".git" && name != "target"
        })
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.is_file() && is_dotted_config_toml(path, &runtime.dotted_dir) {
            let Ok(content) = fs::read_to_string(path) else {
                continue;
            };
            let parsed: toml::Value = match toml::from_str(&content) {
                Ok(v) => v,
                Err(err) => {
                    eprintln!(
                        "Skipping invalid TOML at {}: {err}",
                        runtime.display_path(path).display()
                    );
                    continue;
                }
            };
            let formatted = toml::to_string_pretty(&parsed)
                .wrap_err_with(|| format!("format {}", path.display()))?;

            if content != formatted {
                fs::write(path, &formatted)
                    .wrap_err_with(|| format!("write {}", path.display()))?;
                let display = runtime.display_path(path);
                println!(
                    "Formatted {}",
                    style(&display.to_string_lossy(), "32", runtime)
                );
                formatted_count += 1;
            }
        }
    }

    if formatted_count == 0 {
        println!(
            "{}",
            style(
                "All dotted configuration TOML files are properly formatted.",
                "32",
                runtime
            )
        );
    } else {
        println!(
            "{}",
            style(
                &format!("Successfully formatted {formatted_count} configuration TOML file(s)."),
                "36;1",
                runtime
            )
        );
    }

    Ok(())
}
