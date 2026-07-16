//! StdFs: FileKVFileSystem implementation wrapping std::fs
//!
//! This is the default production implementation that delegates
//! all operations directly to std::fs and memmap2.

use std::any::Any;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write as IoWrite};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::io::{read_dir_to_paths, FileKVFile, FileKVFileSystem, FileMetadata, IoResult, MmapFileSystem, MmapView};

/// Standard filesystem implementation using std::fs
#[derive(Debug, Clone, Copy, Default)]
pub struct StdFs;

impl FileKVFileSystem for StdFs {
    fn clone_as_mmap_fs(&self) -> Option<Arc<dyn MmapFileSystem>> {
        Some(Arc::new(*self))
    }

    fn create_file(&self, path: &Path) -> IoResult<Box<dyn FileKVFile>> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let file = File::create(path)?;
        Ok(Box::new(StdFile(file)))
    }

    fn open_file(&self, path: &Path, read: bool, write: bool, append: bool) -> IoResult<Box<dyn FileKVFile>> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        let mut opts = OpenOptions::new();
        opts.read(read);
        if append {
            opts.write(true).append(true).create(true);
        } else if write {
            opts.write(true).create(true);
        }
        let file = opts.open(path)?;
        Ok(Box::new(StdFile(file)))
    }

    fn read_dir(&self, path: &Path) -> IoResult<Vec<PathBuf>> {
        let entries = std::fs::read_dir(path)?;
        read_dir_to_paths(entries)
    }

    fn create_dir_all(&self, path: &Path) -> IoResult<()> {
        std::fs::create_dir_all(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> IoResult<()> {
        std::fs::rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> IoResult<()> {
        std::fs::remove_file(path)
    }

    fn file_exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn file_metadata(&self, path: &Path) -> IoResult<FileMetadata> {
        let metadata = std::fs::metadata(path)?;
        Ok(FileMetadata::new(metadata.len()))
    }

    fn sync_dir(&self, path: &Path) -> IoResult<()> {
        // On Unix, syncing a directory requires opening it
        let dir = File::open(path)?;
        dir.sync_all()?;
        Ok(())
    }
}

impl MmapFileSystem for StdFs {
    fn mmap(&self, file: &dyn FileKVFile) -> IoResult<Arc<dyn MmapView>> {
        let std_file = file
            .as_any()
            .downcast_ref::<StdFile>()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Expected StdFile"))?;

        let mmap = unsafe { memmap2::MmapOptions::new().map(&std_file.0)? };
        Ok(Arc::new(StdMmap(mmap)))
    }
}

/// Standard file handle wrapper
pub struct StdFile(File);

impl FileKVFile for StdFile {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.0.read(buf)
    }

    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.0.write(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> IoResult<()> {
        self.0.write_all(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.0.flush()
    }

    fn sync_all(&self) -> IoResult<()> {
        self.0.sync_all()
    }

    fn try_clone(&self) -> IoResult<Box<dyn FileKVFile>> {
        Ok(Box::new(StdFile(self.0.try_clone()?)))
    }

    fn metadata(&self) -> IoResult<FileMetadata> {
        let metadata = self.0.metadata()?;
        Ok(FileMetadata::new(metadata.len()))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Standard mmap view
pub struct StdMmap(memmap2::Mmap);

impl MmapView for StdMmap {
    fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }

    fn len(&self) -> usize {
        self.0.len()
    }
}
