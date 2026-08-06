/// CLI Command: `workspace init [git_url]`
///
/// Initializes or clones a dotted workspace, creates the new config/layout,
/// and never creates `[local].toml`.
use color_eyre::eyre::{Result, bail};
use std::fs;

use crate::commands::lib::settings_path;
use crate::types::{
    AGENTS_MD, ARTIFACTS_DIR, ColorSection, ConfigSection, DEFAULT_AGENTS_MD, DEFAULT_MEMORY_MD,
    DottedFile, MEMORY_MD, Runtime, SettingsFile,
};
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

    let agents_md = runtime.dotted_dir.join(AGENTS_MD);
    if !agents_md.exists() {
        fs::write(&agents_md, DEFAULT_AGENTS_MD)?;
    }
    let memory_md = runtime.dotted_dir.join(MEMORY_MD);
    if !memory_md.exists() {
        fs::write(&memory_md, DEFAULT_MEMORY_MD)?;
    }

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
        crate::utils::style(
            &runtime
                .display_path(&runtime.dotted_dir)
                .display()
                .to_string(),
            "32",
            runtime
        )
    );
    Ok(())
}
