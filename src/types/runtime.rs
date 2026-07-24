use color_eyre::eyre::{ContextCompat, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::types::{DEFAULT_BACKUP_DIR, DEFAULT_DOTTED_DIR, DOTTED_TOML, LOCAL_TOML, SETTINGS_DIR};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
struct LocalFile {
    device: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct Runtime {
    pub(crate) dotted_dir: PathBuf,
    pub(crate) home_dir: PathBuf,
    pub(crate) root_dir: PathBuf,
    pub(crate) user: String,
    pub(crate) device: String,
    pub(crate) distro: String,
    pub(crate) no_color: bool,
}

const DEFAULT_DEVICE: &str = "default_device";

impl Runtime {
    pub(crate) fn from_cli(cli: &crate::cli::Cli) -> Result<Self> {
        let user = cli
            .user
            .clone()
            .or_else(|| std::env::var("SUDO_USER").ok())
            .or_else(|| std::env::var("USER").ok())
            .unwrap_or_else(|| "user".to_string());
        let home_dir = cli
            .home_dir
            .clone()
            .or_else(|| get_passwd_home(&user))
            .or_else(dirs::home_dir)
            .context("could not resolve home directory")?;
        let dotted_dir = cli
            .dotted_dir
            .clone()
            .unwrap_or_else(|| home_dir.join(DEFAULT_DOTTED_DIR));
        let root_dir = cli.root_dir.clone().unwrap_or_else(|| PathBuf::from("/"));
        let device = cli
            .device
            .clone()
            .or_else(|| {
                let path = dotted_dir.join(LOCAL_TOML);
                fs::read_to_string(path)
                    .ok()
                    .and_then(|content| toml::from_str::<LocalFile>(&content).ok())
                    .and_then(|local| local.device)
            })
            .or_else(|| std::env::var("HOSTNAME").ok())
            .or_else(|| {
                fs::read_to_string(root_dir.join("etc/hostname"))
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| DEFAULT_DEVICE.to_string());
        let distro = cli
            .distro
            .clone()
            .unwrap_or_else(crate::plan::detect_distro);
        let no_color = cli.no_color;

        Ok(Self {
            dotted_dir,
            home_dir,
            root_dir,
            user,
            device,
            distro,
            no_color,
        })
    }

    pub(crate) fn dotted_path(&self) -> PathBuf {
        self.dotted_dir.join(DOTTED_TOML)
    }

    pub(crate) fn settings_root(&self) -> PathBuf {
        self.dotted_dir.join(SETTINGS_DIR)
    }

    pub(crate) fn backup_root(&self) -> PathBuf {
        self.home_dir.join(DEFAULT_BACKUP_DIR)
    }

    pub(crate) fn resolve_tilde(&self, value: &str) -> PathBuf {
        if value == "~" {
            return self.home_dir.clone();
        }
        if let Some(rest) = value.strip_prefix("~/") {
            return self.home_dir.join(rest);
        }
        PathBuf::from(value)
    }

    pub(crate) fn resolve_abs_target(&self, target: &Path) -> PathBuf {
        if self.root_dir == Path::new("/") {
            target.to_path_buf()
        } else {
            let stripped = target.strip_prefix("/").unwrap_or(target);
            self.root_dir.join(stripped)
        }
    }

    pub(crate) fn display_path(&self, path: &Path) -> PathBuf {
        if let Ok(rest) = path.strip_prefix(&self.home_dir) {
            return PathBuf::from("~").join(rest);
        }
        if let Ok(rest) = path.strip_prefix(&self.dotted_dir) {
            if rest.as_os_str().is_empty() {
                return PathBuf::from(".");
            }
            let mut components = rest.components();
            if components
                .next()
                .is_some_and(|component| component.as_os_str() == "[artifacts]")
            {
                return PathBuf::from("artifacts").join(components.as_path());
            }
            return components.as_path().to_path_buf();
        }
        if self.root_dir != Path::new("/")
            && let Ok(rest) = path.strip_prefix(&self.root_dir)
        {
            return PathBuf::from("/").join(rest);
        }
        path.to_path_buf()
    }
}

pub(crate) fn get_passwd_home(username: &str) -> Option<PathBuf> {
    if let Ok(content) = fs::read_to_string("/etc/passwd") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 6 && parts[0] == username {
                return Some(PathBuf::from(parts[5]));
            }
        }
    }
    None
}
