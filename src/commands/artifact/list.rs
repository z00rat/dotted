/// CLI Command: `artifact list [--filter <text>] [--raw]`
///
/// What it does:
/// Lists all discovered artifacts in the workspace, along with their enabling status, revision, repository, and description.
///
/// Variations:
/// 1. `--filter` provided: Matches against artifact ID, description, or repository name.
/// 2. `--raw` / `-r` provided: Prints only the list of matching artifact IDs (one per line) to stdout, suitable for scripting.
/// 3. `--state enabled|disabled` limits candidates by effective enablement state.
///
/// Decisions & Logic Branches:
/// - If `raw` is true, outputs raw IDs and exits.
/// - Determines the artifact's enabling status ([enabled] or [disabled]) by checking if it exists in the settings' `enable` list and is not in the `disable` list.
/// - Formats the output inside a dynamic table if `raw` is false, applying terminal colors if enabled.
use color_eyre::eyre::Result;

use crate::plan::{discover_artifacts, load_settings};
use crate::types::Runtime;

pub fn run(runtime: &Runtime, filter: Option<&str>, raw: bool, state: Option<&str>) -> Result<()> {
    let matches_filter = |s: &str| -> bool {
        if let Some(f) = filter {
            s.to_lowercase().contains(&f.to_lowercase())
        } else {
            true
        }
    };

    let settings = load_settings(runtime)?;
    let artifacts = discover_artifacts(runtime)?
        .values()
        .filter(|art| {
            let enabled = settings.enable.contains(&art.id) && !settings.disable.contains(&art.id);
            let state_match = state.is_none_or(|value| (value == "enabled") == enabled);
            state_match
                && (matches_filter(&art.id)
                    || matches_filter(&art.description)
                    || matches_filter(&art.repo))
        })
        .cloned()
        .collect::<Vec<_>>();

    if raw {
        for art in artifacts {
            println!("{}", art.id);
        }
        return Ok(());
    }

    crate::utils::print_banner("LIST ARTIFACT RESOURCES", runtime);
    if artifacts.is_empty() {
        println!("No matching artifacts.");
        return Ok(());
    }

    let mut table = comfy_table::Table::new();
    table
        .load_preset(comfy_table::presets::UTF8_FULL)
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic)
        .set_header(vec!["STATUS", "ARTIFACT ID", "REVISION", "DESCRIPTION"]);

    for art in artifacts {
        let enabled = settings.enable.contains(&art.id) && !settings.disable.contains(&art.id);
        let status_cell = if !runtime.no_color {
            if enabled {
                comfy_table::Cell::new("[enabled]").fg(comfy_table::Color::Green)
            } else {
                comfy_table::Cell::new("[disabled]").fg(comfy_table::Color::DarkGrey)
            }
        } else if enabled {
            comfy_table::Cell::new("[enabled]")
        } else {
            comfy_table::Cell::new("[disabled]")
        };
        table.add_row(vec![
            status_cell,
            comfy_table::Cell::new(art.id),
            comfy_table::Cell::new(art.revision.to_string()),
            comfy_table::Cell::new(art.description),
        ]);
    }
    println!("{table}");
    Ok(())
}
