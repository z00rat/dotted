/// CLI Command: `workspace init [git_url]`
///
/// Initializes or clones a dotted workspace, creates the new config/layout,
/// and never creates `[local].toml`.
use color_eyre::eyre::{Result, bail};
use std::fs;

use crate::commands::lib::settings_path;
use crate::types::{ARTIFACTS_DIR, ColorSection, ConfigSection, DottedFile, Runtime, SettingsFile};
use crate::utils::run_git;

fn write_config_files(runtime: &Runtime) -> Result<()> {
    if !runtime.dotted_path().exists() {
        crate::types::write_toml(
            &runtime.dotted_path(),
            &DottedFile {
                config: ConfigSection::default(),
                repos: Vec::new(),
                color: ColorSection::default(),
            },
        )?;
    }
    let gitignore = runtime.dotted_dir.join(".gitignore");
    if !gitignore.exists() {
        fs::write(gitignore, crate::types::DEFAULT_GITIGNORE)?;
    }
    fs::create_dir_all(runtime.dotted_dir.join(ARTIFACTS_DIR))?;
    let fallback = runtime.settings_root().join("[device]").join("[user].toml");
    if !fallback.exists() {
        crate::types::write_toml(&fallback, &SettingsFile::default())?;
    }
    let settings = settings_path(runtime);
    if !settings.exists() {
        crate::types::write_toml(&settings, &SettingsFile::default())?;
    }
    Ok(())
}

pub fn run(runtime: &Runtime, git_url: Option<String>) -> Result<()> {
    if runtime.dotted_path().exists() || runtime.dotted_dir.join(".git").exists() {
        bail!(
            "dotted workspace already exists at {}",
            runtime.display_path(&runtime.dotted_dir).display()
        );
    }
    if let Some(url) = git_url {
        if runtime.dotted_dir.exists() {
            bail!(
                "{} already exists",
                runtime.display_path(&runtime.dotted_dir).display()
            );
        }
        let parent = runtime
            .dotted_dir
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        run_git(
            parent,
            [
                "clone",
                url.as_str(),
                runtime.dotted_dir.to_string_lossy().as_ref(),
            ],
        )?;
    } else {
        fs::create_dir_all(&runtime.dotted_dir)?;
        run_git(&runtime.dotted_dir, ["init"])?;
    }
    write_config_files(runtime)?;
    println!(
        "initialized {}",
        runtime.display_path(&runtime.dotted_dir).display()
    );
    Ok(())
}
