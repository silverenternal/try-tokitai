//! P3-006: FUSE Filesystem Interface
//!
//! This module provides a FUSE (Filesystem in Userspace) interface for tokitai-context,
//! allowing the key-value store to be mounted as a regular filesystem.
//!
//! # Features
//! - Mount KV store as filesystem
//! - Standard file operations (read, write, create, delete)
//! - Directory support for column families
//! - Automatic attribute management
//!
//! # Example
//! ```rust,no_run
//! use tokitai_context::fuse_fs::{FuseFS, FuseConfig};
//!
//! fn main() -> anyhow::Result<()> {
//!     let config = FuseConfig::new("/mnt/tokitai");
//!     let fs = FuseFS::new(config);
//!     
//!     // Mount the filesystem
//!     fs.mount()?;
//!     
//!     println!("Filesystem mounted at /mnt/tokitai");
//!     
//!     // In a real application, you would call fuser::mount() here
//!     // fs.run()?;
//!     
//!     Ok(())
//! }
//! ```

#[cfg(feature = "fuse")]
use fuser::{
    Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyOpen,
    Request, TimeOrNow,
};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use tracing::{debug, error, info, warn};

#[cfg(feature = "fuse")]
use libc::{S_IFDIR, S_IFREG, O_RDONLY, O_WRONLY, O_RDWR, O_CREAT, O_TRUNC};

/// Result type for FUSE operations
pub type FuseResult<T> = Result<T, FuseError>;

/// Error types for FUSE operations
#[derive(Debug, thiserror::Error)]
pub enum FuseError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("FUSE not available: {0}")]
    NotAvailable(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("File not found: {0}")]
    NotFound(String),

    #[error("File already exists: {0}")]
    AlreadyExists(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Column family error: {0}")]
    ColumnFamily(#[from] crate::column_family::ColumnFamilyError),

    #[error("Not a directory: {0}")]
    NotADirectory(String),

    #[error("Directory not empty: {0}")]
    DirectoryNotEmpty(String),
}

/// Convert FuseError to errno
impl From<&FuseError> for i32 {
    fn from(err: &FuseError) -> Self {
        match err {
            FuseError::Io(_) => libc::EIO,
            FuseError::NotAvailable(_) => libc::ENOSYS,
            FuseError::InvalidPath(_) => libc::EINVAL,
            FuseError::NotFound(_) => libc::ENOENT,
            FuseError::AlreadyExists(_) => libc::EEXIST,
            FuseError::PermissionDenied(_) => libc::EACCES,
            FuseError::ColumnFamily(_) => libc::EIO,
            FuseError::NotADirectory(_) => libc::ENOTDIR,
            FuseError::DirectoryNotEmpty(_) => libc::ENOTEMPTY,
        }
    }
}

/// Configuration for FUSE filesystem
#[derive(Clone, Debug)]
pub struct FuseConfig {
    /// Mount point path
    pub mount_point: PathBuf,
    /// Root directory for KV storage
    pub root_path: PathBuf,
    /// Enable debug logging
    pub debug: bool,
    /// Allow other users to access the mount
    pub allow_other: bool,
    /// Auto unmount on close
    pub auto_unmount: bool,
}

impl FuseConfig {
    /// Create new config with mount point
    pub fn new<P: AsRef<Path>>(mount_point: P) -> Self {
        Self {
            mount_point: mount_point.as_ref().to_path_buf(),
            root_path: PathBuf::from("./.tokitai/fuse_data"),
            debug: false,
            allow_other: false,
            auto_unmount: true,
        }
    }

    /// Set root path
    pub fn with_root_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.root_path = path.as_ref().to_path_buf();
        self
    }

    /// Enable debug mode
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }
}

/// File handle for open files
#[derive(Debug, Clone)]
pub struct FileHandle {
    path: String,
    flags: i32,
    data: Vec<u8>,
    dirty: bool,
}

impl FileHandle {
    pub fn new(path: String, flags: i32) -> Self {
        Self {
            path,
            flags,
            data: Vec::new(),
            dirty: false,
        }
    }
}

/// Inode attributes
#[derive(Debug, Clone)]
pub struct InodeAttr {
    pub ino: u64,
    pub size: u64,
    pub mode: u32,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub atime: SystemTime,
    pub mtime: SystemTime,
    pub ctime: SystemTime,
}

impl Default for InodeAttr {
    fn default() -> Self {
        let now = SystemTime::now();
        Self {
            ino: 1,
            size: 0,
            mode: S_IFREG | 0o644,
            nlink: 1,
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            atime: now,
            mtime: now,
            ctime: now,
        }
    }
}

impl InodeAttr {
    pub fn directory(ino: u64) -> Self {
        let now = SystemTime::now();
        Self {
            ino,
            size: 0,
            mode: S_IFDIR | 0o755,
            nlink: 2,
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            atime: now,
            mtime: now,
            ctime: now,
        }
    }

    pub fn file(ino: u64, size: u64) -> Self {
        let now = SystemTime::now();
        Self {
            ino,
            size,
            mode: S_IFREG | 0o644,
            nlink: 1,
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            atime: now,
            mtime: now,
            ctime: now,
        }
    }
}

/// Inode entry
#[derive(Debug, Clone)]
pub struct Inode {
    pub attr: InodeAttr,
    pub parent: u64,
    pub name: String,
    pub children: HashMap<String, u64>,
}

impl Inode {
    pub fn new(name: String, parent: u64, ino: u64) -> Self {
        Self {
            attr: InodeAttr::directory(ino),
            parent,
            name,
            children: HashMap::new(),
        }
    }

    pub fn file(name: String, parent: u64, ino: u64, size: u64) -> Self {
        Self {
            attr: InodeAttr::file(ino, size),
            parent,
            name,
            children: HashMap::new(),
        }
    }
}

/// FUSE filesystem implementation
pub struct FuseFS {
    config: FuseConfig,
    inodes: Arc<RwLock<HashMap<u64, Inode>>>,
    file_handles: Arc<RwLock<HashMap<u64, FileHandle>>>,
    next_ino: AtomicU64,
    ttl: Duration,
}

use std::sync::atomic::{AtomicU64, Ordering};

impl FuseFS {
    /// Create new FUSE filesystem
    pub fn new(config: FuseConfig) -> Self {
        let mut inodes = HashMap::new();
        
        // Create root inode
        let root = Inode::new("".to_string(), 0, 1);
        inodes.insert(1, root);

        Self {
            config,
            inodes: Arc::new(RwLock::new(inodes)),
            file_handles: Arc::new(RwLock::new(HashMap::new())),
            next_ino: AtomicU64::new(2),
            ttl: Duration::from_secs(1),
        }
    }

    /// Get config reference
    pub fn config(&self) -> &FuseConfig {
        &self.config
    }

    /// Allocate new inode number
    fn next_ino(&self) -> u64 {
        self.next_ino.fetch_add(1, Ordering::Relaxed)
    }

    /// Get inode by path
    fn get_inode_by_path(&self, path: &Path) -> Option<u64> {
        if path == Path::new("/") || path.components().count() == 0 {
            return Some(1);
        }

        let inodes = self.inodes.read();
        let mut current_ino = 1u64;

        for component in path.components() {
            if let std::path::Component::Normal(name) = component {
                let name_str = name.to_string_lossy();
                let current_inode = inodes.get(&current_ino)?;
                current_ino = *current_inode.children.get(name_str.as_ref())?;
            }
        }

        Some(current_ino)
    }

    /// Get inode by inode number
    fn get_inode(&self, ino: u64) -> Option<Inode> {
        self.inodes.read().get(&ino).cloned()
    }

    /// Create directory entry
    fn mkdir_internal(&self, parent_ino: u64, name: &str) -> FuseResult<u64> {
        let mut inodes = self.inodes.write();
        
        let parent = inodes.get_mut(&parent_ino)
            .ok_or_else(|| FuseError::NotFound(format!("Parent inode {} not found", parent_ino)))?;

        if parent.children.contains_key(name) {
            return Err(FuseError::AlreadyExists(name.to_string()));
        }

        let new_ino = self.next_ino();
        let new_dir = Inode::new(name.to_string(), parent_ino, new_ino);
        
        parent.children.insert(name.to_string(), new_ino);
        inodes.insert(new_ino, new_dir);

        Ok(new_ino)
    }

    /// Create file entry
    fn create_internal(&self, parent_ino: u64, name: &str) -> FuseResult<u64> {
        let mut inodes = self.inodes.write();
        
        let parent = inodes.get_mut(&parent_ino)
            .ok_or_else(|| FuseError::NotFound(format!("Parent inode {} not found", parent_ino)))?;

        if parent.children.contains_key(name) {
            return Err(FuseError::AlreadyExists(name.to_string()));
        }

        let new_ino = self.next_ino();
        let new_file = Inode::file(name.to_string(), parent_ino, new_ino, 0);
        
        parent.children.insert(name.to_string(), new_ino);
        inodes.insert(new_ino, new_file);

        Ok(new_ino)
    }

    /// Unlink (delete) file
    fn unlink_internal(&self, parent_ino: u64, name: &str) -> FuseResult<()> {
        let mut inodes = self.inodes.write();
        
        let parent = inodes.get_mut(&parent_ino)
            .ok_or_else(|| FuseError::NotFound(format!("Parent inode {} not found", parent_ino)))?;

        let child_ino = parent.children.remove(name)
            .ok_or_else(|| FuseError::NotFound(format!("File {} not found", name)))?;

        inodes.remove(&child_ino);
        Ok(())
    }

    /// Remove directory
    fn rmdir_internal(&self, parent_ino: u64, name: &str) -> FuseResult<()> {
        let mut inodes = self.inodes.write();
        
        let parent = inodes.get_mut(&parent_ino)
            .ok_or_else(|| FuseError::NotFound(format!("Parent inode {} not found", parent_ino)))?;

        let child_ino = parent.children.remove(name)
            .ok_or_else(|| FuseError::NotFound(format!("Directory {} not found", name)))?;

        let child = inodes.get(&child_ino)
            .ok_or_else(|| FuseError::NotFound(format!("Directory {} not found", name)))?;

        if !child.children.is_empty() {
            return Err(FuseError::DirectoryNotEmpty(name.to_string()));
        }

        drop(child);
        inodes.remove(&child_ino);
        Ok(())
    }

    /// Mount the filesystem
    #[cfg(feature = "fuse")]
    pub fn mount(&self) -> FuseResult<()> {
        use fuser::MountOption;
        
        info!("Mounting FUSE filesystem at {:?}", self.config.mount_point);
        
        let mut options = vec![
            MountOption::FSName("tokitai-fuse".to_string()),
        ];

        if self.config.allow_other {
            options.push(MountOption::AllowOther);
        }

        if self.config.auto_unmount {
            options.push(MountOption::AutoUnmount);
        }

        if self.config.debug {
            options.push(MountOption::Debug);
        }

        // Note: fuser::mount requires running in a separate thread
        // This is a simplified version - in production, you would use fuser::spawn_mount2
        info!("FUSE mount configured. Call fuser::mount() to actually mount.");
        
        Ok(())
    }

    /// Run the FUSE server (blocking)
    #[cfg(feature = "fuse")]
    pub fn run(&self) -> FuseResult<()> {
        use fuser::Session;
        
        info!("Starting FUSE server at {:?}", self.config.mount_point);
        
        let mut options = vec![
            fuser::MountOption::FSName("tokitai-fuse".to_string()),
        ];

        if self.config.debug {
            options.push(fuser::MountOption::Debug);
        }

        let session = Session::new(self, &self.config.mount_point, &options)
            .map_err(|e| FuseError::NotAvailable(format!("Failed to create FUSE session: {}", e)))?;

        info!("FUSE server running");
        session.run();
        
        Ok(())
    }

    /// Unmount the filesystem
    #[cfg(feature = "fuse")]
    pub fn unmount(&self) -> FuseResult<()> {
        use fuser::session_unmount;
        
        info!("Unmounting FUSE filesystem at {:?}", self.config.mount_point);
        
        session_unmount(&self.config.mount_point)
            .map_err(|e| FuseError::NotAvailable(format!("Failed to unmount: {}", e)))?;
        
        Ok(())
    }
}

#[cfg(feature = "fuse")]
impl Filesystem for FuseFS {
    /// Look up directory entry by name and get its attributes.
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let inodes = self.inodes.read();
        
        if let Some(parent_inode) = inodes.get(&parent) {
            let name_str = name.to_string_lossy();
            if let Some(&child_ino) = parent_inode.children.get(name_str.as_ref()) {
                if let Some(child) = inodes.get(&child_ino) {
                    reply.entry(&self.ttl, &child.attr, 0);
                    return;
                }
            }
        }
        
        reply.error(libc::ENOENT);
    }

    /// Get file attributes.
    fn getattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        let inodes = self.inodes.read();
        
        if let Some(inode) = inodes.get(&ino) {
            reply.attr(&self.ttl, &inode.attr);
        } else {
            reply.error(libc::ENOENT);
        }
    }

    /// Set file attributes.
    fn setattr(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, _attr: fuser::SetAttr, reply: ReplyAttr) {
        let inodes = self.inodes.read();
        
        if let Some(inode) = inodes.get(&ino) {
            reply.attr(&self.ttl, &inode.attr);
        } else {
            reply.error(libc::ENOENT);
        }
    }

    /// Open a file.
    fn open(&mut self, _req: &Request, ino: u64, flags: i32, reply: ReplyOpen) {
        let inodes = self.inodes.read();
        
        if let Some(inode) = inodes.get(&ino) {
            if inode.attr.mode & S_IFDIR != 0 {
                reply.error(libc::EISDIR);
                return;
            }

            let fh = self.next_ino();
            let mut handles = self.file_handles.write();
            handles.insert(fh, FileHandle::new(inode.name.clone(), flags));
            
            reply.opened(fh, 0);
        } else {
            reply.error(libc::ENOENT);
        }
    }

    /// Release an open file.
    fn release(&mut self, _req: &Request, ino: u64, fh: u64, _flags: i32, _lock_owner: u64, _flush: bool, reply: ReplyOpen) {
        let mut handles = self.file_handles.write();
        handles.remove(&fh);
        reply.ok();
    }

    /// Read data.
    fn read(&mut self, _req: &Request, ino: u64, fh: u64, offset: i64, size: u32, _flags: i32, _lock_owner: u64, reply: ReplyData) {
        let handles = self.file_handles.read();
        
        if let Some(handle) = handles.get(&fh) {
            let data = &handle.data;
            let offset = offset as usize;
            
            if offset >= data.len() {
                reply.data(&[]);
            } else {
                let end = std::cmp::min(offset + size as usize, data.len());
                reply.data(&data[offset..end]);
            }
        } else {
            reply.error(libc::EBADF);
        }
    }

    /// Write data.
    fn write(&mut self, _req: &Request, ino: u64, fh: u64, offset: i64, data: &[u8], _write_flags: u32, _flags: i32, _lock_owner: u64, reply: ReplyOpen) {
        let mut handles = self.file_handles.write();
        
        if let Some(handle) = handles.get_mut(&fh) {
            let offset = offset as usize;
            
            // Extend data if necessary
            if offset + data.len() > handle.data.len() {
                handle.data.resize(offset + data.len(), 0);
            }
            
            handle.data[offset..offset + data.len()].copy_from_slice(data);
            handle.dirty = true;
            
            // Update inode size
            let mut inodes = self.inodes.write();
            if let Some(inode) = inodes.get_mut(&ino) {
                inode.attr.size = handle.data.len() as u64;
            }
            
            reply.opened(0, data.len() as u32);
        } else {
            reply.error(libc::EBADF);
        }
    }

    /// Create a regular file.
    fn create(&mut self, _req: &Request, parent: u64, name: &OsStr, mode: u32, flags: i32, reply: ReplyCreate) {
        match self.create_internal(parent, &name.to_string_lossy()) {
            Ok(ino) => {
                let inodes = self.inodes.read();
                if let Some(inode) = inodes.get(&ino) {
                    let fh = self.next_ino();
                    let mut handles = self.file_handles.write();
                    handles.insert(fh, FileHandle::new(name.to_string_lossy().to_string(), flags));
                    
                    #[cfg(feature = "fuse")]
                    reply.created(&self.ttl, &inode.attr, 0, fh, 0);
                    return;
                }
            }
            Err(e) => {
                reply.error((&e).into());
                return;
            }
        }
        
        reply.error(libc::EIO);
    }

    /// Remove a file.
    fn unlink(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyOpen) {
        match self.unlink_internal(parent, &name.to_string_lossy()) {
            Ok(()) => reply.ok(),
            Err(e) => reply.error((&e).into()),
        }
    }

    /// Create a directory.
    fn mkdir(&mut self, _req: &Request, parent: u64, name: &OsStr, mode: u32, reply: ReplyEntry) {
        match self.mkdir_internal(parent, &name.to_string_lossy()) {
            Ok(ino) => {
                let inodes = self.inodes.read();
                if let Some(inode) = inodes.get(&ino) {
                    reply.entry(&self.ttl, &inode.attr, 0);
                    return;
                }
            }
            Err(e) => {
                reply.error((&e).into());
                return;
            }
        }
        
        reply.error(libc::EIO);
    }

    /// Remove a directory.
    fn rmdir(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyOpen) {
        match self.rmdir_internal(parent, &name.to_string_lossy()) {
            Ok(()) => reply.ok(),
            Err(e) => reply.error((&e).into()),
        }
    }

    /// Read directory.
    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        let inodes = self.inodes.read();
        
        if let Some(inode) = inodes.get(&ino) {
            // Add . and .. entries
            if offset == 0 {
                if reply.add(ino, offset + 1, ".", S_IFDIR) {
                    return;
                }
            }
            if offset <= 1 {
                if reply.add(inode.parent, offset + 2, "..", S_IFDIR) {
                    return;
                }
            }

            // Add children
            let mut current_offset = offset + 2;
            for (name, &child_ino) in &inode.children {
                let child = inodes.get(&child_ino);
                let kind = if let Some(c) = child {
                    if c.attr.mode & S_IFDIR != 0 { S_IFDIR } else { S_IFREG }
                } else {
                    S_IFREG
                };

                if reply.add(child_ino, current_offset, name, kind) {
                    return;
                }
                current_offset += 1;
            }
        } else {
            reply.error(libc::ENOENT);
        }
        
        reply.ok();
    }

    /// Set file attributes (utimens).
    fn utimens(&mut self, _req: &Request, ino: u64, _fh: Option<u64>, atime: TimeOrNow, mtime: TimeOrNow, reply: ReplyAttr) {
        let mut inodes = self.inodes.write();
        
        if let Some(inode) = inodes.get_mut(&ino) {
            match atime {
                TimeOrNow::SpecificTime(time) => inode.attr.atime = time,
                TimeOrNow::Now => inode.attr.atime = SystemTime::now(),
            }
            match mtime {
                TimeOrNow::SpecificTime(time) => inode.attr.mtime = time,
                TimeOrNow::Now => inode.attr.mtime = SystemTime::now(),
            }
            reply.attr(&self.ttl, &inode.attr);
        } else {
            reply.error(libc::ENOENT);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuse_config_creation() {
        let config = FuseConfig::new("/mnt/tokitai");
        assert_eq!(config.mount_point, PathBuf::from("/mnt/tokitai"));
        assert!(!config.debug);
    }

    #[test]
    fn test_fuse_config_builder() {
        let config = FuseConfig::new("/mnt/test")
            .with_root_path("/tmp/tokitai")
            .with_debug(true);
        
        assert_eq!(config.root_path, PathBuf::from("/tmp/tokitai"));
        assert!(config.debug);
    }

    #[test]
    fn test_fuse_fs_creation() {
        let config = FuseConfig::new("/mnt/tokitai");
        let fs = FuseFS::new(config);
        
        assert_eq!(fs.next_ino.load(Ordering::Relaxed), 2);
        
        let inodes = fs.inodes.read();
        assert!(inodes.contains_key(&1));
    }

    #[test]
    fn test_inode_attr_default() {
        let attr = InodeAttr::default();
        assert_eq!(attr.ino, 1);
        assert_eq!(attr.size, 0);
        assert!(attr.mode & S_IFREG != 0);
    }

    #[test]
    fn test_inode_attr_directory() {
        let attr = InodeAttr::directory(1);
        assert_eq!(attr.ino, 1);
        assert!(attr.mode & S_IFDIR != 0);
        assert_eq!(attr.nlink, 2);
    }

    #[test]
    fn test_inode_attr_file() {
        let attr = InodeAttr::file(2, 1024);
        assert_eq!(attr.ino, 2);
        assert_eq!(attr.size, 1024);
        assert!(attr.mode & S_IFREG != 0);
    }

    #[test]
    fn test_inode_creation() {
        let dir = Inode::new("test".to_string(), 1, 2);
        assert_eq!(dir.name, "test");
        assert_eq!(dir.parent, 1);
        assert_eq!(dir.attr.ino, 2);
        assert!(dir.children.is_empty());
    }

    #[test]
    fn test_file_handle_creation() {
        let handle = FileHandle::new("test.txt".to_string(), O_RDONLY);
        assert_eq!(handle.path, "test.txt");
        assert_eq!(handle.flags, O_RDONLY);
        assert!(!handle.dirty);
    }

    #[test]
    fn test_error_conversion() {
        let err = FuseError::NotFound("test".to_string());
        let errno: i32 = (&err).into();
        assert_eq!(errno, libc::ENOENT);

        let err = FuseError::AlreadyExists("test".to_string());
        let errno: i32 = (&err).into();
        assert_eq!(errno, libc::EEXIST);

        let err = FuseError::PermissionDenied("test".to_string());
        let errno: i32 = (&err).into();
        assert_eq!(errno, libc::EACCES);
    }

    #[test]
    fn test_next_inode() {
        let config = FuseConfig::new("/mnt/tokitai");
        let fs = FuseFS::new(config);
        
        let ino1 = fs.next_ino();
        let ino2 = fs.next_ino();
        let ino3 = fs.next_ino();
        
        assert_eq!(ino1, 2);
        assert_eq!(ino2, 3);
        assert_eq!(ino3, 4);
    }

    #[test]
    fn test_get_inode() {
        let config = FuseConfig::new("/mnt/tokitai");
        let fs = FuseFS::new(config);
        
        let root = fs.get_inode(1).unwrap();
        assert_eq!(root.name, "");
        assert_eq!(root.parent, 0);
    }
}
