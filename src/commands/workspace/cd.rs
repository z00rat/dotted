/// CLI Command: `workspace cd`
///
/// Starts `$SHELL` (falling back to `/bin/sh`) with the working directory set
/// to the resolved dotted workspace.
use color_eyre::eyre::{Context, Result, bail};
use std::process::Command;

use crate::types::Runtime;

pub fn run(runtime: &Runtime) -> Result<()> {
    if !runtime.dotted_dir.is_dir() {
        bail!(
            "dotted workspace does not exist at {}",
            runtime.display_path(&runtime.dotted_dir).display()
        );
    }
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    Command::new(&shell)
        .current_dir(&runtime.dotted_dir)
        .status()
        .with_context(|| format!("start shell {shell}"))
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                bail!("shell exited with {status}")
            }
        })
}
