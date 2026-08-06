use color_eyre::eyre::Result;
use std::fs;
use std::path::Path;
use walkdir::DirEntry;

pub(crate) fn preserve_source_permissions(source: &Path, target: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(source)?.permissions().mode();
        let mut permissions = fs::metadata(target)?.permissions();
        permissions.set_mode(mode);
        if let Err(e) = fs::set_permissions(target, permissions) {
            if e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(1) {
                let status = std::process::Command::new("sudo")
                    .args([
                        "chmod",
                        &format!("{:o}", mode & 0o777),
                        &target.to_string_lossy(),
                    ])
                    .status()?;
                if !status.success() {
                    color_eyre::eyre::bail!(
                        "failed to set permissions as root on {}",
                        target.display()
                    );
                }
            } else {
                return Err(e.into());
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (source, target);
    }
    Ok(())
}

pub(crate) fn backup_file(
    runtime: &crate::types::Runtime,
    target: &Path,
    display_target: &Path,
) -> Result<()> {
    let relative = display_target.strip_prefix("/").unwrap_or(display_target);
    let backup = runtime
        .backup_root()
        .join(chrono::Utc::now().timestamp().to_string())
        .join(relative);
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(target, backup)?;
    Ok(())
}

pub(crate) fn cmp_dir_entries(a: &fs::DirEntry, b: &fs::DirEntry) -> std::cmp::Ordering {
    let path_a = a.path();
    let path_b = b.path();
    let is_dir_a = path_a.is_dir();
    let is_dir_b = path_b.is_dir();

    if is_dir_a != is_dir_b {
        return is_dir_b.cmp(&is_dir_a);
    }

    let name_a = a.file_name().to_string_lossy().to_lowercase();
    let name_b = b.file_name().to_string_lossy().to_lowercase();
    name_a.cmp(&name_b)
}

pub(crate) fn cmp_walkdir_entries(a: &DirEntry, b: &DirEntry) -> std::cmp::Ordering {
    let is_dir_a = a.file_type().is_dir();
    let is_dir_b = b.file_type().is_dir();

    if is_dir_a != is_dir_b {
        return is_dir_b.cmp(&is_dir_a);
    }

    let name_a = a.file_name().to_string_lossy().to_lowercase();
    let name_b = b.file_name().to_string_lossy().to_lowercase();
    name_a.cmp(&name_b)
}
