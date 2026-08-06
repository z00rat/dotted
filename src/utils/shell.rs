use color_eyre::eyre::{Result, WrapErr, bail};
use std::path::Path;
use std::process::Command;

pub(crate) fn run_git<const N: usize>(dir: &Path, args: [&str; N]) -> Result<()> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .wrap_err_with(|| format!("run git in {}", dir.display()))?;
    if output.status.success() {
        Ok(())
    } else {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        if detail.is_empty() {
            bail!("git failed in {}", dir.display())
        }
        bail!("git failed in {}: {detail}", dir.display())
    }
}
