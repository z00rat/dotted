/// CLI Command: `ignore service list`
///
/// What it does:
/// Lists all systemd service unit names configured in the global settings ignore service list.
///
/// Variations:
/// None.
///
/// Decisions & Logic Branches:
/// - Builds the workspace execution plan to resolve all effective service ignore rules.
/// - Formats ignored services with status styling using shared `crate::ignore` module.
use color_eyre::eyre::Result;

use crate::ignore;
use crate::plan::build_plan;
use crate::types::Runtime;

pub fn run(runtime: &Runtime) -> Result<()> {
    let plan = build_plan(runtime, None)?;
    ignore::list(runtime, "Ignored Services:", &plan.ignored_services);
    Ok(())
}
