/// CLI Command: `ignore service list`
///
/// What it does:
/// Lists all currently ignored systemd service units across global settings and active artifacts.
///
/// Decisions & Logic Branches:
/// - Builds active plan to aggregate `ignored_services` from `settings.toml` and enabled artifact `dotted.toml` configs.
/// - Prints the formatted list of ignored service unit names or `(none)` if empty.
use color_eyre::eyre::Result;

use crate::plan::build_plan;
use crate::types::Runtime;
use crate::utils::style;

pub fn run(runtime: &Runtime) -> Result<()> {
    let plan = build_plan(runtime, None)?;

    println!("{}", style("Ignored Services:", "36;1", runtime));
    if plan.ignored_services.is_empty() {
        println!("  (none)");
    } else {
        for svc in &plan.ignored_services {
            println!("  - {}", style(svc, "90", runtime));
        }
    }
    Ok(())
}
