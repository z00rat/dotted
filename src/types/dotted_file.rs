use crate::types::DEFAULT_ENV_SH;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct DottedFile {
    #[serde(default)]
    pub(crate) config: ConfigSection,
    #[serde(default)]
    #[serde(rename = "repo")]
    pub(crate) repos: Vec<RepoConfig>,
    #[serde(default)]
    pub(crate) color: ColorSection,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct ConfigSection {
    #[serde(default = "default_config_version")]
    pub(crate) v: String,
    #[serde(default = "default_env_path")]
    pub(crate) env_path: Vec<String>,
    #[serde(default, flatten)]
    pub(crate) package_commands: std::collections::HashMap<String, String>,
}

impl Default for ConfigSection {
    fn default() -> Self {
        Self {
            v: default_config_version(),
            env_path: default_env_path(),
            package_commands: default_package_commands(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct ColorSection {
    #[serde(default = "default_color")]
    pub(crate) success: String,
    #[serde(default = "default_color")]
    pub(crate) warning: String,
    #[serde(default = "default_color")]
    pub(crate) error: String,
    #[serde(default = "default_color")]
    pub(crate) info: String,
    #[serde(default = "default_muted_color")]
    pub(crate) muted: String,
    #[serde(default = "default_installed_color")]
    pub(crate) installed: String,
    #[serde(default = "default_color")]
    pub(crate) diff: String,
}

impl Default for ColorSection {
    fn default() -> Self {
        Self {
            success: "green".into(),
            warning: "yellow".into(),
            error: "red".into(),
            info: "cyan".into(),
            muted: "bright-black".into(),
            installed: "blue".into(),
            diff: "yellow".into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct RepoConfig {
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) branch: Option<String>,
    pub(crate) tag: Option<String>,
    pub(crate) revision: Option<String>,
}

pub(crate) fn default_color() -> String {
    "cyan".into()
}
pub(crate) fn default_muted_color() -> String {
    "bright-black".into()
}
pub(crate) fn default_installed_color() -> String {
    "blue".into()
}

pub(crate) fn default_config_version() -> String {
    "0.1.0".to_string()
}

pub(crate) fn default_env_path() -> Vec<String> {
    vec![DEFAULT_ENV_SH.to_string()]
}

pub(crate) fn default_package_commands() -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    map.insert(
        "archlinux".to_string(),
        "sudo pacman -S --needed --noconfirm".to_string(),
    );
    map.insert("fedora".to_string(), "sudo dnf install -y".to_string());
    map.insert("ubuntu".to_string(), "sudo apt-get install -y".to_string());
    map
}
