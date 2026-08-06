/// CLI Command: `ignore package remove [names...]`
///
/// What it does:
/// Removes package names from the global settings ignore package list (`ignore.package`).
///
/// Variations:
/// 1. `names` provided: Removes specified package names from `ignore.package` settings list.
/// 2. `names` empty: Prompts interactively with a select menu of currently ignored packages.
///
/// Decisions & Logic Branches:
/// - Delegates core ignore list removal logic to shared `crate::ignore` module.
/// - Persists updated ignore lists to `settings.toml`.
use color_eyre::eyre::Result;

use crate::ignore::{self, IgnoreKind};
use crate::types::Runtime;

pub fn run(runtime: &Runtime, names: &[String]) -> Result<()> {
    ignore::remove(runtime, IgnoreKind::Package, names)
}
