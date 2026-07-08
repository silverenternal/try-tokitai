use crate::tools::io::error::{IoResult, IoToolError};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathValidationResult {
    pub is_valid: bool,
    pub canonical_path: Option<String>,
    pub error: Option<String>,
    pub suggestion: Option<String>,
}

impl PathValidationResult {
    pub fn into_result(self, path: &str) -> IoResult<String> {
        if self.is_valid {
            Ok(self.canonical_path.unwrap_or_else(|| path.to_string()))
        } else {
            Err(IoToolError::PathValidation {
                message: self
                    .error
                    .unwrap_or_else(|| "unknown path validation error".to_string()),
                path: path.to_string(),
                suggestion: self.suggestion.unwrap_or_else(|| {
                    "check whether the path is inside the active workspace".to_string()
                }),
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub allowed_roots: Vec<PathBuf>,
    pub allow_symlinks: bool,
    pub max_depth: usize,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        #[cfg(feature = "test-allow-all-paths")]
        {
            Self {
                allowed_roots: vec![
                    current_dir.clone(),
                    current_dir.join("sandbox"),
                    current_dir.join("downloads"),
                    current_dir.join("target"),
                    PathBuf::from("/"),
                ],
                allow_symlinks: true,
                max_depth: 100,
            }
        }

        #[cfg(not(feature = "test-allow-all-paths"))]
        {
            Self {
                allowed_roots: vec![
                    current_dir.clone(),
                    current_dir.join("sandbox"),
                    current_dir.join("downloads"),
                ],
                allow_symlinks: false,
                max_depth: 100,
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SecurePathResolver {
    config: SandboxConfig,
}

impl SecurePathResolver {
    pub fn new() -> Self {
        Self {
            config: SandboxConfig::default(),
        }
    }

    #[cfg(any(test, feature = "test-allow-all-paths"))]
    pub fn new_for_tests() -> Self {
        Self {
            config: SandboxConfig {
                allowed_roots: vec![
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                    PathBuf::from("/"),
                ],
                allow_symlinks: true,
                max_depth: 100,
            },
        }
    }

    #[allow(dead_code)]
    pub fn with_config(config: SandboxConfig) -> Self {
        Self { config }
    }

    pub fn resolve(&self, path: &str) -> PathValidationResult {
        if path.trim().is_empty() {
            return PathValidationResult {
                is_valid: false,
                canonical_path: None,
                error: Some("path cannot be empty".to_string()),
                suggestion: Some("provide a path relative to the current workspace".to_string()),
            };
        }

        if path.len() > 4096 {
            return PathValidationResult {
                is_valid: false,
                canonical_path: None,
                error: Some(format!("path is too long ({} > 4096)", path.len())),
                suggestion: Some("shorten the path or use a simpler relative path".to_string()),
            };
        }

        let raw_path = Path::new(path);
        let normalized = match self.normalize_candidate_path(raw_path) {
            Ok(path) => path,
            Err(error) => {
                return PathValidationResult {
                    is_valid: false,
                    canonical_path: None,
                    error: Some(error),
                    suggestion: Some("use a valid path inside the active workspace".to_string()),
                };
            }
        };

        let canonical = match if normalized.exists() {
            self.canonicalize_safe(&normalized)
        } else {
            self.canonicalize_for_nonexistent(&normalized)
        } {
            Ok(path) => path,
            Err(error) => {
                return PathValidationResult {
                    is_valid: false,
                    canonical_path: None,
                    error: Some(error),
                    suggestion: Some(
                        "ensure the parent directory exists inside the active workspace"
                            .to_string(),
                    ),
                };
            }
        };

        if !self.is_within_allowed_roots(&canonical) {
            return PathValidationResult {
                is_valid: false,
                canonical_path: Some(self.display_path(&canonical)),
                error: Some(format!(
                    "path is outside allowed roots: {}",
                    self.display_path(&canonical)
                )),
                suggestion: Some(format!(
                    "allowed roots: {}",
                    self.config
                        .allowed_roots
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            };
        }

        let depth = self.calculate_depth(&canonical);
        if depth > self.config.max_depth {
            return PathValidationResult {
                is_valid: false,
                canonical_path: Some(self.display_path(&canonical)),
                error: Some(format!(
                    "path depth exceeds limit ({} > {})",
                    depth, self.config.max_depth
                )),
                suggestion: Some("use a shallower directory structure".to_string()),
            };
        }

        if !self.config.allow_symlinks && self.path_uses_symlink(&normalized) {
            return PathValidationResult {
                is_valid: false,
                canonical_path: Some(self.display_path(&canonical)),
                error: Some("symbolic links are not allowed".to_string()),
                suggestion: Some("use the real path instead of a symlink".to_string()),
            };
        }

        PathValidationResult {
            is_valid: true,
            canonical_path: Some(self.display_path(&canonical)),
            error: None,
            suggestion: None,
        }
    }

    fn normalize_candidate_path(&self, path: &Path) -> Result<PathBuf, String> {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|err| format!("failed to get current directory: {}", err))?
                .join(path)
        };

        let mut normalized = PathBuf::new();
        for component in absolute.components() {
            match component {
                Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
                Component::RootDir => normalized.push(component.as_os_str()),
                Component::CurDir => {}
                Component::ParentDir => {
                    if !normalized.pop() {
                        return Err("invalid parent directory traversal".to_string());
                    }
                }
                Component::Normal(segment) => normalized.push(segment),
            }
        }

        Ok(normalized)
    }

    fn canonicalize_safe(&self, path: &Path) -> Result<PathBuf, String> {
        let mut visited = HashSet::new();
        let mut current = path.to_path_buf();

        loop {
            match current.canonicalize() {
                Ok(canonical) => {
                    if !visited.insert(canonical.clone()) {
                        return Err("detected a symlink loop".to_string());
                    }
                    return Ok(canonical);
                }
                Err(_) => {
                    if let Some(parent) = current.parent() {
                        current = parent.to_path_buf();
                    } else {
                        return Err("failed to canonicalize path".to_string());
                    }
                }
            }
        }
    }

    fn canonicalize_for_nonexistent(&self, path: &Path) -> Result<PathBuf, String> {
        let mut existing_ancestor = path.to_path_buf();
        let mut suffix: Vec<OsString> = Vec::new();

        while !existing_ancestor.exists() {
            let name = existing_ancestor
                .file_name()
                .ok_or_else(|| format!("path does not exist: {}", path.display()))?;
            suffix.push(name.to_os_string());
            existing_ancestor = existing_ancestor
                .parent()
                .ok_or_else(|| {
                    format!("unable to resolve parent directory for {}", path.display())
                })?
                .to_path_buf();
        }

        let mut canonical = self.canonicalize_safe(&existing_ancestor)?;
        for segment in suffix.iter().rev() {
            canonical.push(segment);
        }
        Ok(canonical)
    }

    fn is_within_allowed_roots(&self, path: &Path) -> bool {
        self.config.allowed_roots.iter().any(|root| {
            if let Ok(canonical_root) = root.canonicalize() {
                path.starts_with(&canonical_root)
            } else {
                path.starts_with(root)
            }
        })
    }

    fn calculate_depth(&self, path: &Path) -> usize {
        path.components().count()
    }

    fn path_uses_symlink(&self, path: &Path) -> bool {
        std::fs::symlink_metadata(path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
    }

    fn display_path(&self, path: &Path) -> String {
        let raw = path.to_string_lossy();
        #[cfg(windows)]
        {
            if let Some(stripped) = raw.strip_prefix(r"\\?\UNC\") {
                return format!(r"\\{}", stripped);
            }
            if let Some(stripped) = raw.strip_prefix(r"\\?\") {
                return stripped.to_string();
            }
        }
        raw.to_string()
    }

    #[allow(dead_code)]
    pub fn add_allowed_root(&mut self, path: PathBuf) {
        if !self.config.allowed_roots.contains(&path) {
            self.config.allowed_roots.push(path);
        }
    }

    #[allow(dead_code)]
    pub fn remove_allowed_root(&mut self, path: &Path) {
        self.config
            .allowed_roots
            .retain(|candidate| candidate != path);
    }
}

impl Default for SecurePathResolver {
    fn default() -> Self {
        Self::new()
    }
}

static GLOBAL_RESOLVER: OnceLock<RwLock<SecurePathResolver>> = OnceLock::new();

pub fn get_global_resolver() -> &'static RwLock<SecurePathResolver> {
    GLOBAL_RESOLVER.get_or_init(|| {
        #[cfg(feature = "test-allow-all-paths")]
        {
            let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            RwLock::new(SecurePathResolver::with_config(SandboxConfig {
                allowed_roots: vec![
                    current_dir.clone(),
                    current_dir.join("sandbox"),
                    current_dir.join("downloads"),
                    current_dir.join("target"),
                    current_dir.join("target").join("test_tmp"),
                    PathBuf::from("/tmp"),
                    PathBuf::from("/"),
                ],
                allow_symlinks: true,
                max_depth: 100,
            }))
        }

        #[cfg(not(feature = "test-allow-all-paths"))]
        {
            RwLock::new(SecurePathResolver::new())
        }
    })
}

#[allow(dead_code)]
pub fn init_global_resolver(config: SandboxConfig) -> bool {
    GLOBAL_RESOLVER
        .set(RwLock::new(SecurePathResolver::with_config(config)))
        .is_ok()
}

pub fn add_allowed_root(path: PathBuf) {
    get_global_resolver().write().add_allowed_root(path);
}

#[allow(dead_code)]
pub fn validate_path(path: &str) -> PathValidationResult {
    get_global_resolver().read().resolve(path)
}

#[allow(dead_code)]
pub fn is_path_safe(path: &str) -> bool {
    get_global_resolver().read().resolve(path).is_valid
}

#[allow(dead_code)]
pub fn validate_path_or_error(path: &str) -> IoResult<String> {
    get_global_resolver().read().resolve(path).into_result(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_rejects_outside_root() {
        let config = SandboxConfig {
            allowed_roots: vec![PathBuf::from("/tmp")],
            allow_symlinks: false,
            max_depth: 100,
        };
        let resolver = SecurePathResolver::with_config(config);
        let result = resolver.resolve("/etc/passwd");
        assert!(!result.is_valid);
    }

    #[test]
    fn test_existing_file_inside_root_is_valid() {
        let tmpdir = tempdir().unwrap();
        let file = tmpdir.path().join("test.txt");
        fs::write(&file, "hello").unwrap();

        let config = SandboxConfig {
            allowed_roots: vec![tmpdir.path().to_path_buf()],
            allow_symlinks: false,
            max_depth: 100,
        };
        let resolver = SecurePathResolver::with_config(config);

        let result = resolver.resolve(&file.to_string_lossy());
        assert!(result.is_valid);
    }

    #[test]
    fn test_nonexistent_child_inside_root_is_valid() {
        let tmpdir = tempdir().unwrap();
        let nested_dir = tmpdir.path().join("experiments");
        fs::create_dir_all(&nested_dir).unwrap();
        let target = nested_dir.join("new_file.py");

        let config = SandboxConfig {
            allowed_roots: vec![tmpdir.path().to_path_buf()],
            allow_symlinks: false,
            max_depth: 100,
        };
        let resolver = SecurePathResolver::with_config(config);

        let result = resolver.resolve(&target.to_string_lossy());
        assert!(result.is_valid);
        assert_eq!(
            result.canonical_path.as_deref(),
            Some(target.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn test_write_file_creates_new_file_inside_allowed_root() {
        use crate::tools::io::file_ops::FileOperations;

        let tmpdir = tempdir().unwrap();
        let nested_dir = tmpdir.path().join("experiments");
        fs::create_dir_all(&nested_dir).unwrap();
        let target = nested_dir.join("hello_workspace.txt");

        let config = SandboxConfig {
            allowed_roots: vec![tmpdir.path().to_path_buf()],
            allow_symlinks: false,
            max_depth: 100,
        };
        let ops = FileOperations::with_resolver(SecurePathResolver::with_config(config));

        let result = ops.write_file(
            target.to_string_lossy().to_string(),
            "workspace scoped write".to_string(),
        );

        assert!(result.is_ok());
        assert_eq!(
            fs::read_to_string(&target).unwrap(),
            "workspace scoped write"
        );
    }

    #[test]
    fn test_relative_traversal_is_rejected() {
        let tmpdir = tempdir().unwrap();
        let config = SandboxConfig {
            allowed_roots: vec![tmpdir.path().to_path_buf()],
            allow_symlinks: false,
            max_depth: 100,
        };
        let resolver = SecurePathResolver::with_config(config);

        let result = resolver.resolve("../../../etc/passwd");
        assert!(!result.is_valid);
    }

    #[test]
    fn test_into_result_success() {
        let tmpdir = tempdir().unwrap();
        let file = tmpdir.path().join("sample.txt");
        fs::write(&file, "hello").unwrap();

        let config = SandboxConfig {
            allowed_roots: vec![tmpdir.path().to_path_buf()],
            allow_symlinks: false,
            max_depth: 100,
        };
        let resolver = SecurePathResolver::with_config(config);

        let validation = resolver.resolve(&file.to_string_lossy());
        assert!(validation.into_result(&file.to_string_lossy()).is_ok());
    }

    #[test]
    fn test_concurrent_path_validation() {
        use std::sync::Arc;
        use std::thread;

        let tmpdir = tempdir().unwrap();
        let config = SandboxConfig {
            allowed_roots: vec![tmpdir.path().to_path_buf()],
            allow_symlinks: false,
            max_depth: 100,
        };
        let resolver = Arc::new(SecurePathResolver::with_config(config));

        let mut handles = Vec::new();
        for index in 0..10 {
            let path = tmpdir.path().join(format!("file_{}.txt", index));
            let resolver = Arc::clone(&resolver);
            handles.push(thread::spawn(move || {
                resolver.resolve(&path.to_string_lossy()).is_valid
            }));
        }

        for handle in handles {
            assert!(handle.join().is_ok());
        }
    }
}
