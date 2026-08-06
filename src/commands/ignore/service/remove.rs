/// CLI Command: `ignore service remove [names...]`
///
/// What it does:
/// Removes systemd unit service names from the global settings ignore service list (`ignore.service`).
///
/// Variations:
/// 1. `names` provided: Removes specified service unit names from `ignore.service` settings list.
/// 2. `names` empty: Prompts interactively with a select menu of currently ignored services.
///
/// Decisions & Logic Branches:
/// - Delegates core ignore list removal logic to shared `crate::ignore` module.
/// - Persists updated ignore lists to `settings.toml`.
use color_eyre::eyre::Result;

use crate::ignore::{self, IgnoreKind};
use crate::types::Runtime;

pub fn run(runtime: &Runtime, names: &[String]) -> Result<()> {
    ignore::remove(runtime, IgnoreKind::Service, names)
}
