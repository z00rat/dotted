/// CLI Command: `artifact disable <artifact_id>`
///
/// What it does:
/// Disables a specific artifact on the current device for the current user.
///
/// Variations:
/// None (requires an exact `artifact_id` argument).
///
/// Decisions & Logic Branches:
/// - Does NOT validate if the `artifact_id` exists in the workspace (allows disabling non-existent or removed artifacts).
/// - Reads the current user's settings file (creating a default template if it does not exist).
/// - Removes the `artifact_id` from the settings' `enable` list and inserts it into the `disable` list.
/// - Writes the updated settings back to the TOML file.
use color_eyre::eyre::Result;
use std::collections::BTreeSet;

use crate::commands::lib::settings_path;
use crate::types::{Runtime, SettingsFile};
use crate::utils::style;

pub fn run(runtime: &Runtime, artifact_id: &str) -> Result<()> {
    crate::utils::print_banner("DISABLING ARTIFACTS", runtime);
    let path = settings_path(runtime);
    let mut file: SettingsFile = if path.exists() {
        crate::types::read_toml(&path)?
    } else {
        SettingsFile::default()
    };
    let mut enable: BTreeSet<String> = file.artifacts.enable.into_iter().collect();
    let mut disable: BTreeSet<String> = file.artifacts.disable.into_iter().collect();

    enable.remove(artifact_id);
    disable.insert(artifact_id.to_string());

    file.artifacts.enable = enable.into_iter().collect();
    file.artifacts.disable = disable.into_iter().collect();
    crate::types::write_toml(&path, &file)?;
    println!("Disabled {}", style(artifact_id, "33;1", runtime));
    Ok(())
}
