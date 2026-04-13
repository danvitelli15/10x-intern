use std::path::Path;

use anyhow::Result;

pub fn create_file(path: &Path, content: &str) -> Result<()> {
    if path.exists() {
        anyhow::bail!("file already exists: {}", path.display());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_file_writes_content_to_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("output.txt");
        create_file(&path, "hello").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn create_file_errors_if_file_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("output.txt");
        std::fs::write(&path, "existing").unwrap();
        assert!(create_file(&path, "new content").is_err());
    }

    #[test]
    fn create_file_creates_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b/c/output.txt");
        create_file(&path, "hello").unwrap();
        assert!(path.exists());
    }
}
