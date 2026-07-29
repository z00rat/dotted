use color_eyre::eyre::{Result, WrapErr};
use std::fs;

use crate::types::PlannedFile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileChange {
    New,
    Changed,
}

pub(crate) fn read_current(file: &PlannedFile) -> Result<Option<Vec<u8>>> {
    match fs::read(&file.target) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => {
            Err(error).wrap_err_with(|| format!("read {}", file.display_target.display()))
        }
    }
}

pub(crate) fn classify(file: &PlannedFile) -> Result<Option<FileChange>> {
    let Some(current) = read_current(file)? else {
        return Ok(Some(FileChange::New));
    };
    Ok((current != file.bytes).then_some(FileChange::Changed))
}

pub(crate) fn has_changes(plan: &crate::types::Plan, artifact_id: &str) -> Result<bool> {
    plan.files
        .iter()
        .filter(|file| file.artifact_id == artifact_id)
        .try_fold(false, |has_changes, file| {
            Ok(has_changes || classify(file)?.is_some())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn planned_file(target: PathBuf, bytes: &[u8]) -> PlannedFile {
        PlannedFile {
            artifact_id: "/test".to_owned(),
            source: PathBuf::from("source"),
            target: target.clone(),
            display_target: target,
            text: None,
            bytes: bytes.to_vec(),
        }
    }

    #[test]
    fn classifies_missing_and_equal_files() {
        let temp = TempDir::new().unwrap_or_else(|error| panic!("temp dir: {error}"));
        let target = temp.path().join("file");
        let file = planned_file(target.clone(), b"content");
        assert_eq!(
            classify(&file).unwrap_or_else(|error| panic!("classify: {error}")),
            Some(FileChange::New)
        );

        std::fs::write(&target, b"content").unwrap_or_else(|error| panic!("write: {error}"));
        assert_eq!(
            classify(&file).unwrap_or_else(|error| panic!("classify: {error}")),
            None
        );
    }

    #[test]
    fn classifies_changed_files() {
        let temp = TempDir::new().unwrap_or_else(|error| panic!("temp dir: {error}"));
        let target = temp.path().join("file");
        std::fs::write(&target, b"old").unwrap_or_else(|error| panic!("write: {error}"));
        let file = planned_file(target, b"new");
        assert_eq!(
            classify(&file).unwrap_or_else(|error| panic!("classify: {error}")),
            Some(FileChange::Changed)
        );
    }
}
