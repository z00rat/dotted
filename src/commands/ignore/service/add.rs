/// CLI Command: `ignore service add [names...]`
///
/// What it does:
/// Appends systemd unit service names to the global settings ignore service list (`ignore.service`).
///
/// Variations:
/// 1. `names` provided: Adds specified service unit names to `ignore.service` settings list.
/// 2. `names` empty: Prompts interactively for a service unit name to ignore.
///
/// Decisions & Logic Branches:
/// - Delegates core ignore list management to shared `crate::ignore` module.
/// - Sorts service ignore entries and persists changes to `settings.toml`.
use color_eyre::eyre::Result;

use crate::ignore::{self, IgnoreKind};
use crate::types::Runtime;

pub fn run(runtime: &Runtime, names: &[String]) -> Result<()> {
    ignore::add(runtime, IgnoreKind::Service, names)
}
