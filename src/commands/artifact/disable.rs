/// CLI Command: `artifact disable <artifact_ids...>`
///
/// What it does:
/// Disables one or more artifacts on the current device for the current user.
///
/// Variations:
/// None (requires one or more `artifact_id` arguments).
///
/// Decisions & Logic Branches:
/// - Does NOT validate if the `artifact_ids` exist in the workspace (allows disabling non-existent or removed artifacts).
/// - Reads the current user's settings file (creating a default template if it does not exist).
/// - Removes all `artifact_ids` from the settings' `enable` list and inserts them into the `disable` list.
/// - Writes the updated settings back to the TOML file.
use color_eyre::eyre::Result;
use std::collections::BTreeSet;

use crate::commands::lib::settings_path;
use crate::types::{Runtime, SettingsFile};
use crate::utils::style;

pub fn run(runtime: &Runtime, artifact_ids: &[String]) -> Result<()> {
    crate::utils::print_banner("DISABLING ARTIFACTS", runtime);
    let path = settings_path(runtime);
    let mut file: SettingsFile = if path.exists() {
        crate::types::read_toml(&path)?
    } else {
        SettingsFile::default()
    };
    let mut enable: BTreeSet<String> = file.artifacts.enable.into_iter().collect();
    let mut disable: BTreeSet<String> = file.artifacts.disable.into_iter().collect();

    for artifact_id in artifact_ids {
        enable.remove(artifact_id);
        disable.insert(artifact_id.to_string());
    }

    file.artifacts.enable = enable.into_iter().collect();
    file.artifacts.disable = disable.into_iter().collect();
    crate::types::write_toml(&path, &file)?;
    for artifact_id in artifact_ids {
        println!("Disabled {}", style(artifact_id, "33;1", runtime));
    }
    Ok(())
}
