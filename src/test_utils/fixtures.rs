//! 测试夹具 (Fixtures)
//!
//! 提供可复用的测试环境和资源

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// 创建临时项目目录
///
/// 返回的 TempDir 在 drop 时会自动清理
pub fn temp_project_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

/// 创建包含测试文件的临时目录
///
/// # Arguments
/// * `files` - 文件名和内容的键值对
///
/// # Returns
/// 临时目录路径
pub fn temp_dir_with_files(files: &[(&str, &str)]) -> TempDir {
    let temp_dir = temp_project_dir();

    for (filename, content) in files {
        let file_path = temp_dir.path().join(filename);

        // 如果文件名包含路径分隔符，创建父目录
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent directories");
        }

        fs::write(&file_path, content).expect("Failed to write test file");
    }

    temp_dir
}

/// 创建测试用的配置文件
pub fn create_test_config_file(dir: &Path, config_name: &str, content: &str) -> PathBuf {
    let config_path = dir.join(config_name);
    fs::write(&config_path, content).expect("Failed to write config file");
    config_path
}

/// 创建测试用的 Git 仓库结构
pub fn create_test_git_structure(dir: &Path) -> PathBuf {
    let git_dir = dir.join(".git");
    fs::create_dir_all(&git_dir).expect("Failed to create .git directory");

    // 创建基本的 Git 文件
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main").expect("Failed to write HEAD");
    fs::write(
        git_dir.join("config"),
        "[core]\n\trepositoryformatversion = 0",
    )
    .expect("Failed to write config");

    dir.to_path_buf()
}

/// 创建测试用的上下文目录结构
pub fn create_test_context_structure(base_dir: &Path) -> PathBuf {
    let context_dir = base_dir.join(".atlas").join("context");
    fs::create_dir_all(&context_dir).expect("Failed to create context directory");

    // 创建 branches 目录
    fs::create_dir_all(context_dir.join("branches")).expect("Failed to create branches directory");

    // 创建 snapshots 目录
    fs::create_dir_all(context_dir.join("snapshots"))
        .expect("Failed to create snapshots directory");

    context_dir
}

/// 读取测试资源文件
///
/// # Arguments
/// * `relative_path` - 相对于 tests/fixtures 目录的路径
///
/// # Returns
/// 文件内容
pub fn read_fixture(relative_path: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixture_path = Path::new(manifest_dir)
        .join("tests")
        .join("fixtures")
        .join(relative_path);

    fs::read_to_string(&fixture_path)
        .unwrap_or_else(|_| panic!("Failed to read fixture: {:?}", fixture_path))
}

/// 创建测试用的 JSON 文件
pub fn create_test_json_file(dir: &Path, filename: &str, json_content: &str) -> PathBuf {
    let file_path = dir.join(filename);
    fs::write(&file_path, json_content).expect("Failed to write JSON file");
    file_path
}

/// 创建测试用的 Markdown 文件
pub fn create_test_markdown_file(dir: &Path, filename: &str, content: &str) -> PathBuf {
    let file_path = dir.join(filename);
    fs::write(&file_path, content).expect("Failed to write Markdown file");
    file_path
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_temp_project_dir() {
        let temp_dir = temp_project_dir();
        assert!(temp_dir.path().exists());
        assert!(temp_dir.path().is_dir());
        // TempDir 会在 drop 时自动清理
    }

    #[test]
    fn test_temp_dir_with_files() {
        let temp_dir = temp_dir_with_files(&[
            ("file1.txt", "content 1"),
            ("file2.txt", "content 2"),
            ("subdir/file3.txt", "content 3"),
        ]);

        assert!(temp_dir.path().join("file1.txt").exists());
        assert!(temp_dir.path().join("file2.txt").exists());
        assert!(temp_dir.path().join("subdir/file3.txt").exists());

        let content = fs::read_to_string(temp_dir.path().join("file1.txt")).unwrap();
        assert_eq!(content, "content 1");
    }

    #[test]
    fn test_create_test_config_file() {
        let temp_dir = temp_project_dir();
        let config_path =
            create_test_config_file(temp_dir.path(), "test.toml", "key = \"value\"\nnumber = 42");

        assert!(config_path.exists());
        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("key = \"value\""));
    }

    #[test]
    fn test_create_test_git_structure() {
        let temp_dir = temp_project_dir();
        create_test_git_structure(temp_dir.path());

        assert!(temp_dir.path().join(".git").exists());
        assert!(temp_dir.path().join(".git/HEAD").exists());
        assert!(temp_dir.path().join(".git/config").exists());
    }

    #[test]
    fn test_create_test_context_structure() {
        let temp_dir = temp_project_dir();
        create_test_context_structure(temp_dir.path());

        let context_path = temp_dir.path().join(".atlas/context");
        assert!(context_path.exists());
        assert!(context_path.join("branches").exists());
        assert!(context_path.join("snapshots").exists());
    }
}
