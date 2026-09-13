use std::path::{Path, PathBuf};

use super::StorageProvider;
use crate::error::AppError;

/// Default `StorageProvider` — grava direto num diretório do disco local (`app_data_dir()` em
/// produção). Mesmo comportamento em disco que `commands/property.rs`/`commands/thesis.rs` já
/// tinham antes desta fatia, só que por trás da trait em vez de `std::fs::*` espalhado.
pub struct LocalFSProvider {
    base_dir: PathBuf,
}

impl LocalFSProvider {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    fn resolve(&self, relative_path: &str) -> PathBuf {
        if relative_path.is_empty() {
            self.base_dir.clone()
        } else {
            self.base_dir.join(relative_path)
        }
    }
}

fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) -> Result<(), AppError> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            walk(&path, base, out)?;
        } else {
            let relative = path.strip_prefix(base).unwrap_or(&path);
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

impl StorageProvider for LocalFSProvider {
    fn write(&self, relative_path: &str, content: &[u8]) -> Result<(), AppError> {
        let dest = self.resolve(relative_path);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(dest, content)?;
        Ok(())
    }

    fn read(&self, relative_path: &str) -> Result<Vec<u8>, AppError> {
        Ok(std::fs::read(self.resolve(relative_path))?)
    }

    fn list(&self, prefix: &str) -> Result<Vec<String>, AppError> {
        let root = self.resolve(prefix);
        let mut out = Vec::new();
        if root.is_dir() {
            walk(&root, &self.base_dir, &mut out)?;
        } else if root.is_file() {
            out.push(prefix.to_string());
        }
        Ok(out)
    }

    fn delete(&self, relative_path: &str) -> Result<(), AppError> {
        let path = self.resolve(relative_path);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    fn delete_prefix(&self, prefix: &str) -> Result<(), AppError> {
        let path = self.resolve(prefix);
        if path.exists() {
            std::fs::remove_dir_all(path)?;
        }
        Ok(())
    }

    fn local_path(&self, relative_path: &str) -> Option<PathBuf> {
        Some(self.resolve(relative_path))
    }

    fn export_all(&self) -> Result<Vec<(String, Vec<u8>)>, AppError> {
        self.list("")?
            .into_iter()
            .map(|path| {
                let bytes = self.read(&path)?;
                Ok((path, bytes))
            })
            .collect()
    }

    fn import_all(&self, files: Vec<(String, Vec<u8>)>) -> Result<(), AppError> {
        for (path, bytes) in files {
            self.write(&path, &bytes)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;
    use rand::RngCore;

    fn temp_provider() -> (LocalFSProvider, PathBuf) {
        let dir = std::env::temp_dir().join(format!("anchor_storage_test_{:08x}", OsRng.next_u32()));
        (LocalFSProvider::new(dir.clone()), dir)
    }

    #[test]
    fn write_then_read_round_trips() {
        let (provider, dir) = temp_provider();
        provider.write("asset_attachments/1/a.pdf", b"hello").unwrap();
        assert_eq!(provider.read("asset_attachments/1/a.pdf").unwrap(), b"hello");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn delete_is_idempotent_for_missing_file() {
        let (provider, dir) = temp_provider();
        provider.delete("asset_attachments/1/missing.pdf").unwrap();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn delete_prefix_removes_whole_subtree() {
        let (provider, dir) = temp_provider();
        provider.write("thesis_attachments/9/a.pdf", b"a").unwrap();
        provider.write("thesis_attachments/9/b.pdf", b"b").unwrap();
        provider.delete_prefix("thesis_attachments/9").unwrap();
        assert!(provider.list("thesis_attachments/9").unwrap().is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn export_all_then_import_all_recreates_every_file() {
        let (source, source_dir) = temp_provider();
        source.write("asset_attachments/1/a.pdf", b"a").unwrap();
        source.write("asset_attachments/2/b.pdf", b"b").unwrap();

        let (target, target_dir) = temp_provider();
        let exported = source.export_all().unwrap();
        target.import_all(exported).unwrap();

        assert_eq!(target.read("asset_attachments/1/a.pdf").unwrap(), b"a");
        assert_eq!(target.read("asset_attachments/2/b.pdf").unwrap(), b"b");

        std::fs::remove_dir_all(&source_dir).ok();
        std::fs::remove_dir_all(&target_dir).ok();
    }

    #[test]
    fn local_path_resolves_under_base_dir() {
        let (provider, dir) = temp_provider();
        let path = provider.local_path("asset_attachments/1/a.pdf").unwrap();
        assert_eq!(path, dir.join("asset_attachments/1/a.pdf"));
    }
}
