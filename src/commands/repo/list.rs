/// CLI Command: `repo list`
///
/// What it does:
/// Lists all repositories configured in `dotted.toml` alongside their source location and the number of artifacts they contain.
///
/// Variations:
/// Lists all configured repositories.
///
/// Decisions & Logic Branches:
/// - Loads the `dotted.toml` and discovers all available artifacts to count how many belong to each repository.
/// - Reports each repository's configured Git URL and discovered artifact count.
/// - Formats results inside a dynamic table.
use color_eyre::eyre::Result;

use crate::commands::lib::{configured_repos, load_dotted};
use crate::plan::discover_artifacts;
use crate::types::Runtime;

pub fn run(runtime: &Runtime) -> Result<()> {
    let dotted = load_dotted(runtime)?;
    let discovered = discover_artifacts(runtime)?;
    let repos = configured_repos(&dotted);
    let matching_repos: Vec<_> = repos.into_iter().collect();
    if matching_repos.is_empty() {
        println!("No matching repositories.");
        return Ok(());
    }

    let mut table = comfy_table::Table::new();
    table
        .load_preset(comfy_table::presets::UTF8_FULL)
        .set_content_arrangement(comfy_table::ContentArrangement::Dynamic)
        .set_header(vec!["REPOSITORY", "SOURCE", "ARTIFACTS"]);

    for repo in matching_repos {
        let count = discovered
            .values()
            .filter(|artifact| artifact.repo == repo.name)
            .count();
        table.add_row(vec![repo.name, repo.url, count.to_string()]);
    }
    println!("{table}");
    Ok(())
}
