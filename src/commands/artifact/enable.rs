/// CLI Command: `artifact enable <artifact_ids...>`
///
/// What it does:
/// Enables one or more artifacts on the current device for the current user.
///
/// Variations:
/// None (requires one or more `artifact_id` arguments).
///
/// Decisions & Logic Branches:
/// - Fails if any specified `artifact_id` does not exist in the workspace.
/// - Reads the current user's settings file (creating a default template if it does not exist).
/// - Inserts all `artifact_ids` into the settings' `enable` list and removes them from the `disable` list.
/// - Writes the updated settings back to the TOML file.
use color_eyre::eyre::{Result, bail};
use std::collections::BTreeSet;

use crate::commands::lib::settings_path;
use crate::plan::discover_artifacts;
use crate::types::{Runtime, SettingsFile};
use crate::utils::style;

pub fn run(runtime: &Runtime, artifact_ids: &[String]) -> Result<()> {
    crate::utils::print_banner("ENABLING ARTIFACTS", runtime);
    let artifacts = discover_artifacts(runtime)?;
    for artifact_id in artifact_ids {
        if !artifacts.contains_key(artifact_id) {
            bail!("artifact not found: {artifact_id}");
        }
    }
    let path = settings_path(runtime);
    let mut file: SettingsFile = if path.exists() {
        crate::types::read_toml(&path)?
    } else {
        SettingsFile::default()
    };
    let mut enable: BTreeSet<String> = file.artifacts.enable.into_iter().collect();
    let mut disable: BTreeSet<String> = file.artifacts.disable.into_iter().collect();

    for artifact_id in artifact_ids {
        enable.insert(artifact_id.clone());
        disable.remove(artifact_id);
    }

    file.artifacts.enable = enable.into_iter().collect();
    file.artifacts.disable = disable.into_iter().collect();
    crate::types::write_toml(&path, &file)?;
    for artifact_id in artifact_ids {
        println!("Enabled {}", style(artifact_id, "32;1", runtime));
    }
    Ok(())
}
