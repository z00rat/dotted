use color_eyre::eyre::{ContextCompat, Result, WrapErr, anyhow, bail};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

use crate::types::{
    ABOUT_TOML, AboutFile, Artifact, BIN_TOML, BinFile, DOTTED_TOML, DottedFile, DownloadInstall,
    DownloadSource, DownloadSpec, FALLBACK_DEVICE, FALLBACK_USER, Plan, PlannedDownload,
    PlannedFile, Runtime, SETTINGS_DIR, Settings,
};

pub(crate) fn load_settings(runtime: &Runtime) -> Result<Settings> {
    let mut settings = Settings::empty();
    let layers = [
        (FALLBACK_DEVICE, FALLBACK_USER),
        (FALLBACK_DEVICE, runtime.user.as_str()),
        (runtime.device.as_str(), FALLBACK_USER),
        (runtime.device.as_str(), runtime.user.as_str()),
    ];

    for (device, user) in layers {
        let path = runtime
            .settings_root()
            .join(device)
            .join(format!("{user}.toml"));
        if path.exists() {
            settings.merge_file(crate::types::read_toml(&path)?);
        }
    }

    Ok(settings)
}

pub(crate) fn discover_artifacts(runtime: &Runtime) -> Result<BTreeMap<String, Artifact>> {
    let mut artifacts = BTreeMap::new();
    if !runtime.dotted_dir.exists() {
        return Ok(artifacts);
    }

    let dotted: DottedFile = crate::types::read_toml(&runtime.dotted_dir.join(DOTTED_TOML))?;
    let configured: BTreeSet<String> = dotted.repos.into_iter().map(|repo| repo.name).collect();

    for entry in fs::read_dir(&runtime.dotted_dir)
        .wrap_err_with(|| format!("read {}", runtime.dotted_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let folder_name = entry.file_name().to_string_lossy().to_string();
        if folder_name == SETTINGS_DIR {
            continue;
        }
        if folder_name != crate::types::ARTIFACTS_DIR && !configured.contains(&folder_name) {
            continue;
        }
        let repo_name = if folder_name == crate::types::ARTIFACTS_DIR {
            "artifacts".to_string()
        } else {
            folder_name
        };
        let about_path = path.join(ABOUT_TOML);
        if !about_path.exists() {
            continue;
        }
        let about: AboutFile = crate::types::read_toml(&about_path)?;
        for (name, metadata) in about.about {
            let dir = path.join(&name);
            if !dir.is_dir() {
                continue;
            }
            let bin_path = dir.join(BIN_TOML);
            let bin = if bin_path.exists() {
                crate::types::read_toml(&bin_path)?
            } else {
                BinFile::default()
            };
            let id = if repo_name == "artifacts" {
                format!("/{name}")
            } else {
                format!("{repo_name}/{name}")
            };
            artifacts.insert(
                id.clone(),
                Artifact {
                    id,
                    repo: repo_name.clone(),
                    name,
                    dir,
                    revision: metadata.r,
                    description: metadata.description,
                    bin,
                },
            );
        }
    }

    Ok(artifacts)
}

fn collect_enabled_artifacts(runtime: &Runtime, only: Option<&str>) -> Result<Vec<Artifact>> {
    let settings = load_settings(runtime)?;
    let discovered = discover_artifacts(runtime)?;
    let ids: Vec<String> = match only {
        Some(id) => vec![id.to_string()],
        None => settings
            .enable
            .difference(&settings.disable)
            .cloned()
            .collect(),
    };
    let mut artifacts = Vec::new();
    for id in ids {
        let artifact = discovered
            .get(&id)
            .cloned()
            .ok_or_else(|| anyhow!("enabled artifact {id} was not found"))?;
        artifacts.push(artifact);
    }
    artifacts.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(artifacts)
}

fn build_planned_files(
    runtime: &Runtime,
    artifacts: &[Artifact],
    settings: &Settings,
) -> Result<Vec<PlannedFile>> {
    let mut files = Vec::new();
    let mut seen_targets: HashMap<PathBuf, String> = HashMap::new();
    for artifact in artifacts {
        for file in artifact_files(artifact)? {
            let relative = file.strip_prefix(&artifact.dir)?;
            let (target, display_target) = map_artifact_path(runtime, relative)?;
            if let Some(first) = seen_targets.insert(display_target.clone(), artifact.id.clone()) {
                bail!(
                    "duplicate target path {} from {first} and {}",
                    display_target.display(),
                    artifact.id
                );
            }
            let bytes = fs::read(&file).wrap_err_with(|| format!("read {}", file.display()))?;
            let text = text_with_replace(&bytes, &settings.replace);
            let rendered_bytes = text
                .as_ref()
                .map_or_else(|| bytes.clone(), |value| value.as_bytes().to_vec());
            files.push(PlannedFile {
                artifact_id: artifact.id.clone(),
                source: file,
                target,
                display_target,
                text,
                bytes: rendered_bytes,
            });
        }
    }
    Ok(files)
}

fn collect_env(
    artifacts: &[Artifact],
    settings: &Settings,
) -> (BTreeMap<String, String>, Vec<String>) {
    let mut env_map = settings.env.clone();
    let mut env_overrides = Vec::new();
    for artifact in artifacts {
        for (key, value) in &artifact.bin.env {
            if env_map.insert(key.clone(), value.clone()).is_some() {
                env_overrides.push(key.clone());
            }
        }
    }
    (env_map, env_overrides)
}

type PackagesAndDownloads = (
    BTreeMap<String, BTreeSet<String>>,
    BTreeSet<String>,
    Vec<crate::types::PlannedDownload>,
);

fn build_packages_and_downloads(
    runtime: &Runtime,
    artifacts: &[Artifact],
) -> Result<PackagesAndDownloads> {
    let distro = runtime.distro.clone();
    let mut packages: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut flatpaks = BTreeSet::new();
    let mut downloads = Vec::new();
    let arch = normalize_arch(std::env::consts::ARCH);
    for artifact in artifacts {
        if let Some(set) = artifact.bin.distro.get(&distro) {
            packages
                .entry(distro.clone())
                .or_default()
                .extend(set.packages.iter().cloned());
        }
        flatpaks.extend(artifact.bin.flatpak.packages.iter().cloned());
        if let Some(download) = artifact.bin.download.get(&arch) {
            downloads.push(plan_download(runtime, &artifact.id, &arch, download)?);
        }
    }
    Ok((packages, flatpaks, downloads))
}

fn collect_ignored_paths(
    runtime: &Runtime,
    artifacts: &[Artifact],
    settings: &Settings,
) -> (BTreeSet<PathBuf>, BTreeSet<PathBuf>) {
    let mut ignored_folders: BTreeSet<PathBuf> = settings
        .ignore_folders
        .iter()
        .map(|path| runtime.resolve_tilde(path))
        .collect();
    let mut ignored_files: BTreeSet<PathBuf> = settings
        .ignore_files
        .iter()
        .map(|path| runtime.resolve_tilde(path))
        .collect();
    for artifact in artifacts {
        ignored_folders.extend(
            artifact
                .bin
                .ignore
                .folder
                .iter()
                .map(|path| runtime.resolve_tilde(path)),
        );
        ignored_files.extend(
            artifact
                .bin
                .ignore
                .file
                .iter()
                .map(|path| runtime.resolve_tilde(path)),
        );
    }
    (ignored_folders, ignored_files)
}

pub(crate) fn build_plan(runtime: &Runtime, only: Option<&str>) -> Result<Plan> {
    let settings = load_settings(runtime)?;
    let artifacts = collect_enabled_artifacts(runtime, only)?;
    let files = build_planned_files(runtime, &artifacts, &settings)?;
    let (env, env_overrides) = collect_env(&artifacts, &settings);
    let (packages, flatpaks, downloads) = build_packages_and_downloads(runtime, &artifacts)?;
    let (ignored_folders, ignored_files) = collect_ignored_paths(runtime, &artifacts, &settings);

    Ok(Plan {
        artifacts,
        files,
        env,
        env_overrides,
        packages,
        flatpaks,
        downloads,
        ignored_folders,
        ignored_files,
    })
}

pub(crate) fn normalize_arch(arch: &str) -> String {
    match arch {
        "aarch64" => "arm64",
        "x86_64" | "amd64" => "x86_64",
        other => other,
    }
    .to_string()
}

pub(crate) fn plan_download(
    runtime: &Runtime,
    artifact_id: &str,
    arch: &str,
    spec: &DownloadSpec,
) -> Result<PlannedDownload> {
    let source = if let Some(url) = &spec.url {
        DownloadSource::Url(url.clone())
    } else if let Some(zip) = &spec.zip {
        let path = spec
            .path
            .clone()
            .context("download zip entries require a path field")?;
        DownloadSource::Zip {
            url: zip.clone(),
            path,
        }
    } else {
        return Err(color_eyre::eyre::eyre!(
            "download for {artifact_id}/{arch} needs url or zip"
        ));
    };
    let binary_name = spec
        .path
        .as_deref()
        .and_then(|path| Path::new(path).file_name())
        .map_or_else(
            || binary_name_from_source(&source),
            std::borrow::ToOwned::to_owned,
        );
    let display_path = match spec.install {
        DownloadInstall::Local => runtime.home_dir.join(".local/bin").join(&binary_name),
        DownloadInstall::System => PathBuf::from("/usr/local/bin").join(&binary_name),
    };
    let install_path = match spec.install {
        DownloadInstall::Local => display_path.clone(),
        DownloadInstall::System => runtime.resolve_abs_target(&display_path),
    };
    Ok(PlannedDownload {
        artifact_id: artifact_id.to_string(),
        source,
        install: spec.install.clone(),
        install_path,
        display_path,
    })
}

pub(crate) fn binary_name_from_source(source: &DownloadSource) -> std::ffi::OsString {
    match source {
        DownloadSource::Url(url) => Path::new(url)
            .file_name()
            .unwrap_or_else(|| OsStr::new("download"))
            .to_owned(),
        DownloadSource::Zip { path, .. } => Path::new(path)
            .file_name()
            .unwrap_or_else(|| OsStr::new("download"))
            .to_owned(),
    }
}

pub(crate) fn artifact_files(artifact: &Artifact) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(&artifact.dir).sort_by_file_name() {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.file_name() == OsStr::new(BIN_TOML) {
            continue;
        }
        files.push(entry.path().to_path_buf());
    }
    Ok(files)
}

pub(crate) fn map_artifact_path(runtime: &Runtime, relative: &Path) -> Result<(PathBuf, PathBuf)> {
    let mut components = relative.components();
    let first = components
        .next()
        .ok_or_else(|| anyhow!("empty artifact path"))?;
    let first = match first {
        Component::Normal(value) => value.to_string_lossy(),
        _ => bail!("invalid artifact path {}", relative.display()),
    };
    let rest = components.as_path();
    let display = match first.as_ref() {
        "home" => runtime.home_dir.join(rest),
        "root" => PathBuf::from("/root").join(rest),
        other => PathBuf::from("/").join(other).join(rest),
    };
    let target = match first.as_ref() {
        "home" => runtime.home_dir.join(rest),
        "root" => runtime.resolve_abs_target(&PathBuf::from("/root").join(rest)),
        other => runtime.resolve_abs_target(&PathBuf::from("/").join(other).join(rest)),
    };
    Ok((target, display))
}

pub(crate) fn text_with_replace(
    bytes: &[u8],
    replace: &BTreeMap<String, String>,
) -> Option<String> {
    if bytes.contains(&0) {
        return None;
    }
    let mut text = String::from_utf8(bytes.to_vec()).ok()?;
    for (from, to) in replace {
        text = text.replace(from, to);
    }
    Some(text)
}

pub(crate) fn detect_distro() -> String {
    let content = fs::read_to_string("/etc/os-release").unwrap_or_default();
    let mut id_like = String::new();
    for line in content.lines() {
        if let Some(id) = line.strip_prefix("ID=") {
            let normalized = normalize_distro_id(id.trim_matches('"'));
            if normalized != "unknown" {
                return normalized;
            }
        }
        if let Some(value) = line.strip_prefix("ID_LIKE=") {
            id_like = value.trim_matches('"').to_string();
        }
    }
    for id in id_like.split_whitespace() {
        let normalized = normalize_distro_id(id);
        if normalized != "unknown" {
            return normalized;
        }
    }
    "unknown".to_string()
}

pub(crate) fn normalize_distro_id(id: &str) -> String {
    match id {
        "arch" | "archlinux" => "archlinux",
        "ubuntu" => "ubuntu",
        "fedora" => "fedora",
        _ => "unknown",
    }
    .to_string()
}
