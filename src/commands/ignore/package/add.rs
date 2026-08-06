/// CLI Command: `ignore package add [names...]`
///
/// What it does:
/// Appends package names to the global settings ignore package list (`ignore.package`).
///
/// Variations:
/// 1. `names` provided: Adds specified package names to `ignore.package` settings list.
/// 2. `names` empty: Prompts interactively for a package name to ignore.
///
/// Decisions & Logic Branches:
/// - Delegates core ignore list management to shared `crate::ignore` module.
/// - Sorts the package ignore entries and persists changes to `settings.toml`.
use color_eyre::eyre::Result;

use crate::ignore::{self, IgnoreKind};
use crate::types::Runtime;

pub fn run(runtime: &Runtime, names: &[String]) -> Result<()> {
    ignore::add(runtime, IgnoreKind::Package, names)
}
