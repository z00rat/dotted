/// CLI Command: `workspace push`
///
/// Stages, commits, and pushes only the dotted repository using a timestamped
/// `dotted: update ...` message; clean workspaces are left unchanged.
use chrono::Local;
use color_eyre::eyre::Result;

use crate::commands::lib::commit_and_push;
use crate::types::Runtime;

pub fn run(runtime: &Runtime) -> Result<()> {
    if !runtime.dotted_dir.join(".git").exists() {
        println!("No Git repository to push.");
        return Ok(());
    }
    let message = format!(
        "dotted: update {}",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    commit_and_push(&runtime.dotted_dir, &message)
}
