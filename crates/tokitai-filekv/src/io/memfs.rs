//! MemFs: In-memory filesystem implementation for testing
//!
//! Uses `BTreeMap<PathBuf, Vec<u8>>` to simulate a filesystem.
//! No actual disk I/O is performed.

use parking_lot::Mutex;
use std::any::Any;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::io::{FileKVFile, FileKVFileSystem, FileMetadata, IoResult};

/// In-memory filesystem for testing
#[derive(Default)]
pub struct MemFs {
    /// Shared state - allows cloning
    state: Arc<MemFsState>,
}

#[derive(Default)]
struct MemFsState {
    /// Map of path -> file contents
    files: Mutex<BTreeMap<PathBuf, Vec<u8>>>,
    /// Set of directories
    dirs: Mutex<BTreeMap<PathBuf, bool>>,
}

impl Clone for MemFs {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl MemFs {
    pub fn new() -> Self {
        let fs = Self::default();
        // Pre-create root directory entries
        fs.state.dirs.lock().insert(PathBuf::from("/"), true);
        fs
    }

    fn normalize_path(path: &Path) -> PathBuf {
        path.components().collect()
    }

    fn ensure_parent_dirs(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            let mut dirs = self.state.dirs.lock();
            let mut current = PathBuf::new();
            for component in parent.components() {
                current.push(component);
                dirs.entry(current.clone()).or_insert(true);
            }
        }
    }
}

impl FileKVFileSystem for MemFs {
    fn create_file(&self, path: &Path) -> IoResult<Box<dyn FileKVFile>> {
        let path = Self::normalize_path(path);
        self.ensure_parent_dirs(&path);
        let mut files = self.state.files.lock();
        files.insert(path.clone(), Vec::new());
        Ok(Box::new(MemFile::new(self.clone(), path)))
    }

    fn open_file(&self, path: &Path, read: bool, write: bool, append: bool) -> IoResult<Box<dyn FileKVFile>> {
        let path = Self::normalize_path(path);
        let mut files = self.state.files.lock();

        if let Some(existing) = files.get(&path) {
            let content = if append {
                existing.clone()
            } else if write {
                Vec::new() // truncate
            } else {
                existing.clone()
            };
            if write && !append {
                files.insert(path.clone(), content.clone());
            }
            // For read-only opens, start from beginning; for write+append, start at end
            let pos = if read && !write { 0 } else { content.len() };
            Ok(Box::new(MemFile::with_content(self.clone(), path, content, pos)))
        } else {
            // Create new
            self.ensure_parent_dirs(&path);
            files.insert(path.clone(), Vec::new());
            Ok(Box::new(MemFile::new(self.clone(), path)))
        }
    }

    fn read_dir(&self, path: &Path) -> IoResult<Vec<PathBuf>> {
        let path = Self::normalize_path(path);
        let files = self.state.files.lock();
        let dirs = self.state.dirs.lock();
        let mut result = Vec::new();

        // Check if directory exists
        if !dirs.contains_key(&path) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Directory not found: {}", path.display()),
            ));
        }

        let path_str = path.to_string_lossy();
        let prefix = if path_str.is_empty() || path_str == "/" {
            "/".to_string()
        } else {
            format!("{}/", path_str)
        };

        // Collect direct children
        for file_path in files.keys() {
            let file_str = file_path.to_string_lossy();
            if file_str.starts_with(&prefix) {
                let rest = &file_str[prefix.len()..];
                if !rest.contains('/') {
                    result.push(file_path.clone());
                }
            }
        }

        // Also add directory entries
        for dir_path in dirs.keys() {
            let dir_str = dir_path.to_string_lossy();
            if dir_str.starts_with(&prefix) && dir_str != path_str {
                let rest = &dir_str[prefix.len()..];
                if !rest.contains('/') && !files.contains_key(dir_path) {
                    result.push(dir_path.clone());
                }
            }
        }

        result.sort();
        Ok(result)
    }

    fn create_dir_all(&self, path: &Path) -> IoResult<()> {
        let path = Self::normalize_path(path);
        let mut dirs = self.state.dirs.lock();
        let mut current = PathBuf::new();
        for component in path.components() {
            current.push(component);
            dirs.entry(current.clone()).or_insert(true);
        }
        Ok(())
    }

    fn rename(&self, from: &Path, to: &Path) -> IoResult<()> {
        let from = Self::normalize_path(from);
        let to = Self::normalize_path(to);
        let mut files = self.state.files.lock();

        if let Some(content) = files.remove(&from) {
            files.insert(to, content);
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", from.display()),
            ))
        }
    }

    fn remove_file(&self, path: &Path) -> IoResult<()> {
        let path = Self::normalize_path(path);
        let mut files = self.state.files.lock();

        if files.remove(&path).is_some() {
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", path.display()),
            ))
        }
    }

    fn file_exists(&self, path: &Path) -> bool {
        let path = Self::normalize_path(path);
        self.state.files.lock().contains_key(&path)
    }

    fn file_metadata(&self, path: &Path) -> IoResult<FileMetadata> {
        let path = Self::normalize_path(path);
        let files = self.state.files.lock();

        if let Some(content) = files.get(&path) {
            Ok(FileMetadata::new(content.len() as u64))
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", path.display()),
            ))
        }
    }

    fn sync_dir(&self, _path: &Path) -> IoResult<()> {
        // No-op for in-memory FS
        Ok(())
    }
}

/// In-memory file handle
pub struct MemFile {
    fs: MemFs,
    path: PathBuf,
    content: Mutex<Vec<u8>>,
    write_pos: Mutex<usize>,
}

impl MemFile {
    fn new(fs: MemFs, path: PathBuf) -> Self {
        Self {
            fs,
            path,
            content: Mutex::new(Vec::new()),
            write_pos: Mutex::new(0),
        }
    }

    fn with_content(fs: MemFs, path: PathBuf, content: Vec<u8>, pos: usize) -> Self {
        Self {
            fs,
            path,
            content: Mutex::new(content),
            write_pos: Mutex::new(pos),
        }
    }

    /// Sync content back to the filesystem
    fn sync_to_fs(&self) {
        let content = self.content.lock();
        let mut files = self.fs.state.files.lock();
        files.insert(self.path.clone(), content.clone());
    }
}

impl FileKVFile for MemFile {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        let pos = *self.write_pos.lock();
        let content = self.content.lock();
        if pos >= content.len() {
            return Ok(0); // EOF
        }
        let available = &content[pos..];
        let to_read = buf.len().min(available.len());
        buf[..to_read].copy_from_slice(&available[..to_read]);
        *self.write_pos.lock() = pos + to_read;
        Ok(to_read)
    }

    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        let pos = *self.write_pos.lock();
        let mut content = self.content.lock();

        if pos >= content.len() {
            // Append
            content.extend_from_slice(buf);
        } else {
            // Overwrite at position
            let end = pos + buf.len();
            if end > content.len() {
                content.resize(end, 0);
            }
            content[pos..end].copy_from_slice(buf);
        }

        *self.write_pos.lock() = pos + buf.len();
        Ok(buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> IoResult<()> {
        let written = self.write(buf)?;
        if written < buf.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "failed to write whole buffer",
            ));
        }
        Ok(())
    }

    fn flush(&mut self) -> IoResult<()> {
        self.sync_to_fs();
        Ok(())
    }

    fn sync_all(&self) -> IoResult<()> {
        self.sync_to_fs();
        Ok(())
    }

    fn try_clone(&self) -> IoResult<Box<dyn FileKVFile>> {
        let content = self.content.lock().clone();
        let pos = *self.write_pos.lock();
        Ok(Box::new(MemFile::with_content(
            self.fs.clone(),
            self.path.clone(),
            content,
            pos,
        )))
    }

    fn metadata(&self) -> IoResult<FileMetadata> {
        let content = self.content.lock();
        Ok(FileMetadata::new(content.len() as u64))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::FileKVFileSystem;

    #[test]
    fn test_create_and_write() {
        let fs = MemFs::new();
        let mut file = fs.create_file(Path::new("/test/file.txt")).unwrap();
        file.write_all(b"hello").unwrap();
        file.sync_all().unwrap();

        assert!(fs.file_exists(Path::new("/test/file.txt")));
        let meta = fs.file_metadata(Path::new("/test/file.txt")).unwrap();
        assert_eq!(meta.len, 5);
    }

    #[test]
    fn test_read_dir() {
        let fs = MemFs::new();
        fs.create_dir_all(Path::new("/test")).unwrap();
        let mut f1 = fs.create_file(Path::new("/test/a.txt")).unwrap();
        f1.write_all(b"1").unwrap();
        f1.sync_all().unwrap();
        let mut f2 = fs.create_file(Path::new("/test/b.txt")).unwrap();
        f2.write_all(b"2").unwrap();
        f2.sync_all().unwrap();

        let entries = fs.read_dir(Path::new("/test")).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_rename() {
        let fs = MemFs::new();
        let mut f = fs.create_file(Path::new("/old.txt")).unwrap();
        f.write_all(b"data").unwrap();
        f.sync_all().unwrap();

        fs.rename(Path::new("/old.txt"), Path::new("/new.txt")).unwrap();
        assert!(!fs.file_exists(Path::new("/old.txt")));
        assert!(fs.file_exists(Path::new("/new.txt")));
        assert_eq!(fs.file_metadata(Path::new("/new.txt")).unwrap().len, 4);
    }

    #[test]
    fn test_remove_file() {
        let fs = MemFs::new();
        let mut f = fs.create_file(Path::new("/to_delete.txt")).unwrap();
        f.write_all(b"temp").unwrap();
        f.sync_all().unwrap();

        assert!(fs.file_exists(Path::new("/to_delete.txt")));
        fs.remove_file(Path::new("/to_delete.txt")).unwrap();
        assert!(!fs.file_exists(Path::new("/to_delete.txt")));
    }

    #[test]
    fn test_open_nonexistent() {
        let fs = MemFs::new();
        // MemFs creates the file if it doesn't exist (create=true)
        let result = fs.open_file(Path::new("/no_such_file.txt"), true, false, false);
        assert!(result.is_ok()); // creates empty file
    }
}
