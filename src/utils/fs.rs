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

#[cfg(unix)]
pub(crate) fn get_user_uid_gid(runtime: &crate::types::Runtime) -> Option<(u32, u32)> {
    if let (Ok(uid_str), Ok(gid_str)) = (std::env::var("SUDO_UID"), std::env::var("SUDO_GID"))
        && let (Ok(uid), Ok(gid)) = (uid_str.parse::<u32>(), gid_str.parse::<u32>())
    {
        return Some((uid, gid));
    }
    let path = if runtime.dotted_dir.exists() {
        &runtime.dotted_dir
    } else {
        &runtime.home_dir
    };
    if let Ok(meta) = fs::metadata(path) {
        use std::os::unix::fs::MetadataExt;
        let uid = meta.uid();
        let gid = meta.gid();
        if uid != 0 {
            return Some((uid, gid));
        }
    }
    crate::types::runtime::get_passwd_uid_gid(&runtime.user)
}

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn ensure_user_writable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::symlink_metadata(path)
            && !meta.file_type().is_symlink()
        {
            let mut perms = meta.permissions();
            let mode = perms.mode();
            if meta.is_dir() {
                if mode & 0o700 != 0o700 {
                    perms.set_mode(mode | 0o700);
                    let _ = fs::set_permissions(path, perms);
                }
            } else if mode & 0o600 != 0o600 {
                perms.set_mode(mode | 0o600);
                let _ = fs::set_permissions(path, perms);
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

pub(crate) fn chown_path_tree_if_root(runtime: &crate::types::Runtime, path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        if !nix::unistd::geteuid().is_root() {
            return Ok(());
        }
        let Some((uid, gid)) = get_user_uid_gid(runtime) else {
            return Ok(());
        };
        chown_tree(path, uid, gid)?;

        let mut curr = path.to_path_buf();
        while let Some(parent) = curr.parent().map(Path::to_path_buf) {
            if !curr.starts_with(&runtime.dotted_dir) {
                break;
            }
            chown_single(&curr, uid, gid)?;
            if parent == runtime.dotted_dir {
                chown_single(&parent, uid, gid)?;
                break;
            }
            curr = parent;
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (runtime, path);
    }
    Ok(())
}

#[cfg(unix)]
fn chown_single(path: &Path, uid: u32, gid: u32) -> Result<()> {
    if let Err(err) = std::os::unix::fs::lchown(path, Some(uid), Some(gid))
        && err.kind() != std::io::ErrorKind::NotFound
    {
        color_eyre::eyre::bail!("failed to chown {}: {err}", path.display());
    }
    let _ = ensure_user_writable(path);
    Ok(())
}

#[cfg(unix)]
fn chown_tree(path: &Path, uid: u32, gid: u32) -> Result<()> {
    if !path.exists() && fs::symlink_metadata(path).is_err() {
        return Ok(());
    }
    chown_single(path, uid, gid)?;
    if path.is_dir() {
        for entry in walkdir::WalkDir::new(path) {
            let entry = entry?;
            chown_single(entry.path(), uid, gid)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
pub(crate) fn is_root_owned(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    if let Ok(meta) = fs::symlink_metadata(path) {
        meta.uid() == 0
    } else {
        false
    }
}

#[cfg(not(unix))]
pub(crate) fn is_root_owned(_path: &Path) -> bool {
    false
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_ensure_user_writable() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("readonly.txt");
        fs::write(&file, "hello").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&file, fs::Permissions::from_mode(0o400)).unwrap();
        }

        ensure_user_writable(&file).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&file).unwrap().permissions().mode();
            assert_ne!(mode & 0o200, 0);
        }
    }
}
