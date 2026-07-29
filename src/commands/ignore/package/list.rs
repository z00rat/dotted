/// CLI Command: `ignore package list`
///
/// What it does:
/// Lists all currently ignored packages across global settings and active artifacts.
///
/// Decisions & Logic Branches:
/// - Builds active plan to aggregate `ignored_packages` from `settings.toml` and enabled artifact `dotted.toml` configs.
/// - Prints the formatted list of ignored package names or `(none)` if empty.
use color_eyre::eyre::Result;

use crate::plan::build_plan;
use crate::types::Runtime;
use crate::utils::style;

pub fn run(runtime: &Runtime) -> Result<()> {
    let plan = build_plan(runtime, None)?;

    println!("{}", style("Ignored Packages:", "36;1", runtime));
    if plan.ignored_packages.is_empty() {
        println!("  (none)");
    } else {
        for pkg in &plan.ignored_packages {
            println!("  - {}", style(pkg, "90", runtime));
        }
    }
    Ok(())
}
