/// CLI Command: `repo about <name>`
///
/// What it does:
/// Displays metadata and descriptions of artifacts defined in a repository's `about.toml` file.
///
/// Variations:
/// None (requires repository name).
///
/// Decisions & Logic Branches:
/// - Fails if the `about.toml` file is missing in the repository's path.
/// - Reads and parses the repository's `about.toml`.
/// - Prints the list of maintainers and their roles, if specified.
/// - Formats the list of defined artifacts (Name, Revision, Description) inside a dynamic table.
use color_eyre::eyre::{Result, bail};

use crate::types::{ABOUT_TOML, AboutFile, Runtime};
use crate::utils::style;

pub fn run(runtime: &Runtime, name: &str) -> Result<()> {
    crate::utils::print_banner(&format!("REPOSITORY ABOUT DETAILS: {name}"), runtime);
    let about_path = runtime.dotted_dir.join(name).join(ABOUT_TOML);
    if !about_path.exists() {
        bail!(
            "[about].toml does not exist for repository '{name}' at {}",
            about_path.display()
        );
    }

    let about_file: AboutFile = crate::types::read_toml(&about_path)?;

    println!("Repository: {}", style(name, "36;1", runtime));
    println!();

    if !about_file.maintainer.is_empty() {
        println!("{}", style("Maintainers", "1", runtime));
        for (role, contact) in &about_file.maintainer {
            println!("  {role}: {contact}");
        }
        println!();
    }

    println!("{}", style("Artifacts", "1", runtime));
    if about_file.about.is_empty() {
        println!("  No artifacts defined.");
    } else {
        let mut table = comfy_table::Table::new();
        table
            .load_preset(comfy_table::presets::UTF8_FULL)
            .set_content_arrangement(comfy_table::ContentArrangement::Dynamic)
            .set_header(vec!["ARTIFACT NAME", "REVISION", "DESCRIPTION"]);

        for (art_name, entry) in &about_file.about {
            table.add_row(vec![
                art_name.clone(),
                entry.r.to_string(),
                entry.description.clone(),
            ]);
        }
        println!("{table}");
    }

    Ok(())
}
