/// CLI Command: `ignore package list`
///
/// What it does:
/// Lists all package names configured in the global settings ignore package list.
///
/// Variations:
/// None.
///
/// Decisions & Logic Branches:
/// - Builds the workspace execution plan to resolve all effective package ignore rules.
/// - Formats ignored packages with status styling using shared `crate::ignore` module.
use color_eyre::eyre::Result;

use crate::ignore;
use crate::plan::build_plan;
use crate::types::Runtime;

pub fn run(runtime: &Runtime) -> Result<()> {
    let plan = build_plan(runtime, None)?;
    ignore::list(runtime, "Ignored Packages:", &plan.ignored_packages);
    Ok(())
}
