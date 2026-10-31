use anyhow::{Context, Result};
use std::path::Path;

pub fn prepare_store_dir(path: &Path) -> Result<()> {
    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            anyhow::bail!(
                "Store directory must not be a symbolic link: {}",
                path.display()
            );
        }
        if !metadata.is_dir() {
            anyhow::bail!("Store path is not a directory: {}", path.display());
        }
    } else {
        std::fs::create_dir_all(path)
            .with_context(|| format!("Failed to create store directory '{}'", path.display()))?;
    }

    set_mode(path, 0o700)
}

pub fn prepare_private_file(path: &Path) -> Result<()> {
    if let Ok(metadata) = std::fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            anyhow::bail!(
                "Private file must not be a symbolic link: {}",
                path.display()
            );
        }
        if !metadata.is_file() {
            anyhow::bail!("Private path is not a file: {}", path.display());
        }
    } else {
        create_private_file(path)?;
    }

    set_mode(path, 0o600)
}

#[cfg(unix)]
fn create_private_file(path: &Path) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;

    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .with_context(|| format!("Failed to create private file '{}'", path.display()))?;
    Ok(())
}

#[cfg(not(unix))]
fn create_private_file(path: &Path) -> Result<()> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("Failed to create private file '{}'", path.display()))?;
    Ok(())
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
        .with_context(|| format!("Failed to secure permissions for '{}'", path.display()))
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> Result<()> {
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};

    #[test]
    fn private_files_are_owner_only() {
        let dir = std::env::temp_dir().join(format!("tgcli-private-{}", rand::random::<u64>()));
        prepare_store_dir(&dir).unwrap();
        let file = dir.join("session.db");

        prepare_private_file(&file).unwrap();

        let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn private_paths_reject_symbolic_links() {
        let dir = std::env::temp_dir().join(format!("tgcli-symlink-{}", rand::random::<u64>()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("target");
        let link = dir.join("session.db");
        std::fs::write(&target, "secret").unwrap();
        symlink(&target, &link).unwrap();

        let error = prepare_private_file(&link).unwrap_err().to_string();

        assert!(error.contains("symbolic link"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
