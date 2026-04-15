//! I/O Abstraction Layer
//!
//! Defines traits for file system operations, enabling:
//! - Fault injection testing (disk full, random IO errors, delays)
//! - In-memory filesystem for tests (no disk I/O)
//! - Unified sync/async I/O path
//!
//! All file operations in the codebase should go through `FileKVFileSystem`
//! instead of calling `std::fs` directly.

mod stdfs;
pub(crate) mod memfs;
mod fault_inject;

pub use stdfs::StdFs;
pub use memfs::MemFs;
pub use fault_inject::{FaultInjector, FaultRule, FaultStrategy};

use std::any::Any;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub type IoResult<T> = std::io::Result<T>;

/// Core file system trait - all file operations go through this
pub trait FileKVFileSystem: Send + Sync + 'static {
    /// Create a new file (truncate if exists)
    fn create_file(&self, path: &Path) -> IoResult<Box<dyn FileKVFile>>;

    /// Open an existing file with specified access modes
    fn open_file(&self, path: &Path, read: bool, write: bool, append: bool) -> IoResult<Box<dyn FileKVFile>>;

    /// Read directory entries (returns file paths)
    fn read_dir(&self, path: &Path) -> IoResult<Vec<PathBuf>>;

    /// Create directory recursively
    fn create_dir_all(&self, path: &Path) -> IoResult<()>;

    /// Rename/move a file
    fn rename(&self, from: &Path, to: &Path) -> IoResult<()>;

    /// Remove a file
    fn remove_file(&self, path: &Path) -> IoResult<()>;

    /// Check if a file exists
    fn file_exists(&self, path: &Path) -> bool;

    /// Get file metadata (size, etc.)
    fn file_metadata(&self, path: &Path) -> IoResult<FileMetadata>;

    /// Sync directory metadata to disk
    fn sync_dir(&self, path: &Path) -> IoResult<()>;

    /// Returns self as Arc<dyn MmapFileSystem> if supported, None otherwise
    fn clone_as_mmap_fs(&self) -> Option<Arc<dyn MmapFileSystem>> {
        None
    }
}

/// Mmap file system trait - extends FileKVFileSystem with mmap support
///
/// This trait is only implemented by filesystems that support memory-mapped files.
/// `MemFs` does NOT implement this trait since it has no real file descriptors.
pub trait MmapFileSystem: FileKVFileSystem {
    /// Create an mmap view of a file
    fn mmap(&self, file: &dyn FileKVFile) -> IoResult<Arc<dyn MmapView>>;
}

/// Core file handle trait - replaces std::fs::File
pub trait FileKVFile: Send + Sync {
    /// Read bytes from the file into the buffer, returns number of bytes read
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize>;

    /// Read exact number of bytes, returns error if EOF reached early
    fn read_exact(&mut self, mut buf: &mut [u8]) -> IoResult<()> {
        while !buf.is_empty() {
            match self.read(buf)? {
                0 => return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "failed to fill whole buffer")),
                n => buf = &mut buf[n..],
            }
        }
        Ok(())
    }

    /// Write bytes to the file, returns number of bytes written
    fn write(&mut self, buf: &[u8]) -> IoResult<usize>;

    /// Write all bytes to the file
    fn write_all(&mut self, mut buf: &[u8]) -> IoResult<()> {
        while !buf.is_empty() {
            match self.write(buf)? {
                0 => return Err(std::io::Error::new(std::io::ErrorKind::WriteZero, "failed to write whole buffer")),
                n => buf = &buf[n..],
            }
        }
        Ok(())
    }

    /// Flush buffered data to OS
    fn flush(&mut self) -> IoResult<()>;

    /// Sync all data to disk (fsync)
    fn sync_all(&self) -> IoResult<()>;

    /// Clone the file handle (shares the underlying file description)
    fn try_clone(&self) -> IoResult<Box<dyn FileKVFile>>;

    /// Get file metadata
    fn metadata(&self) -> IoResult<FileMetadata>;

    /// Get underlying Any for downcasting (needed for memmap2 interop)
    fn as_any(&self) -> &dyn Any;
}

/// Mmap view trait - provides read-only slice access to file contents
pub trait MmapView: Send + Sync {
    /// Get the mmap as a byte slice
    fn as_slice(&self) -> &[u8];

    /// Get the mmap length
    fn len(&self) -> usize;

    /// Check if mmap is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// File metadata
#[derive(Debug, Clone, Copy)]
pub struct FileMetadata {
    /// File size in bytes
    pub len: u64,
    /// Whether the file exists
    pub exists: bool,
}

impl FileMetadata {
    pub fn new(len: u64) -> Self {
        Self { len, exists: true }
    }

    pub fn not_exists() -> Self {
        Self { len: 0, exists: false }
    }
}

// Implement std::io::Read for Box<dyn FileKVFile>
impl std::io::Read for Box<dyn FileKVFile> {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        (**self).read(buf)
    }
}

// Implement std::io::Write for Box<dyn FileKVFile> so it can be used with BufWriter
impl std::io::Write for Box<dyn FileKVFile> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        // Call FileKVFile::write directly, not std::io::Write::write_all
        (**self).write(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> IoResult<()> {
        // Call FileKVFile::write_all directly, not std::io::Write::write_all
        (**self).write_all(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        (**self).flush()
    }
}

// ─── Helper: convert std::io::Error from std::fs operations ───

/// Convert `std::io::Error` from `std::fs::read_dir` into `Vec<PathBuf>`
pub fn read_dir_to_paths(entries: std::fs::ReadDir) -> IoResult<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry?;
        paths.push(entry.path());
    }
    Ok(paths)
}
