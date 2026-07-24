/// CLI Command: `shell env [shell]`
///
/// What it does:
/// Prints shell export commands for all active environment variables defined in enabled artifacts.
///
/// Variations:
/// 1. `shell` provided: Outputs commands formatted for the specified shell (e.g., `fish` or `bash`).
/// 2. `shell` not provided: Defaults to outputting `bash` syntax.
///
/// Decisions & Logic Branches:
/// - Builds the deployment plan to retrieve environment variables.
/// - Formats the export statements depending on the target shell:
///   - For `fish`: Renders `set -gx KEY 'VALUE'` (escaped).
///   - For other shells (like `bash`/`zsh`): Renders `export KEY='VALUE'` (escaped).
/// - Prints warnings to stderr for any overridden environment variable keys.
use color_eyre::eyre::Result;

use crate::plan::build_plan;
use crate::types::Runtime;

pub fn run(runtime: &Runtime, shell: Option<clap_complete::Shell>) -> Result<()> {
    let plan = build_plan(runtime, None)?;
    let shell = shell.unwrap_or(clap_complete::Shell::Bash);
    for (key, value) in plan.env {
        match shell {
            clap_complete::Shell::Fish => {
                println!("set -gx {key} {}", shell_escape::escape(value.into()));
            }
            _ => {
                println!("export {key}={}", shell_escape::escape(value.into()));
            }
        }
    }
    for key in plan.env_overrides {
        eprintln!("env override: {key}");
    }
    Ok(())
}
