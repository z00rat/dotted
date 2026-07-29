use color_eyre::eyre::Result;

use crate::commands::ignore::common;
use crate::plan::build_plan;
use crate::types::Runtime;

pub fn run(runtime: &Runtime) -> Result<()> {
    let plan = build_plan(runtime, None)?;
    common::list(runtime, "Ignored Packages:", &plan.ignored_packages);
    Ok(())
}
