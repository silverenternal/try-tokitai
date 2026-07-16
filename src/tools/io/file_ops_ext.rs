use super::file_ops::{FileOperations, PatchChange};
use crate::text_encoding::read_text_file;
use crate::tools::io::error::IoToolError;
use crate::tools::io::utils::{
    ensure_file_exists, ensure_is_file, ensure_parent_dir_exists, ensure_path_not_exists,
    validate_single_path,
};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

impl FileOperations {
    pub fn read_file_range(
        &self,
        path: String,
        start_line: usize,
        end_line: usize,
    ) -> Result<Value, Value> {
        let canonical_path =
            validate_single_path(self.resolver(), &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical_path);
        ensure_file_exists(path_obj).map_err(|e| e.to_value())?;
        ensure_is_file(path_obj).map_err(|e| e.to_value())?;

        let content = read_text_file(path_obj).map_err(|e| {
            IoToolError::IoError {
                message: e.to_string(),
                path: Some(canonical_path.clone()),
                operation: "read_file_range".to_string(),
                suggestion: "请检查文件权限或文件是否被其他进程占用".to_string(),
            }
            .to_value()
        })?;

        let lines = content.lines().collect::<Vec<_>>();
        let start = start_line.max(1);
        let end = end_line.max(start);
        let start_idx = start.saturating_sub(1).min(lines.len());
        let end_idx = end.min(lines.len());
        let selected = if start_idx >= end_idx {
            Vec::new()
        } else {
            lines[start_idx..end_idx].to_vec()
        };

        Ok(IoToolError::success_response(
            "read_file_range",
            json!({
                "path": canonical_path,
                "start_line": start,
                "end_line": end,
                "returned_lines": selected.len(),
                "total_lines": lines.len(),
                "truncated": end_idx < end,
                "content": selected.join("\n")
            }),
        ))
    }

    pub fn mkdir(&self, path: String, recursive: Option<bool>) -> Result<Value, Value> {
        let canonical_path =
            validate_single_path(self.resolver(), &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical_path);
        let recursive = recursive.unwrap_or(true);

        if path_obj.exists() {
            if path_obj.is_dir() {
                return Ok(IoToolError::success_response(
                    "mkdir",
                    json!({
                        "path": canonical_path,
                        "recursive": recursive,
                        "created": false,
                        "already_exists": true
                    }),
                ));
            }

            return Err(IoToolError::NotADirectory {
                path: canonical_path,
                suggestion: "目标路径已存在且不是目录，请更换路径或删除冲突文件".to_string(),
            }
            .to_value());
        }

        let result = if recursive {
            fs::create_dir_all(path_obj)
        } else {
            fs::create_dir(path_obj)
        };

        result.map_err(|e| {
            IoToolError::DirCreationFailed {
                path: canonical_path.clone(),
                message: e.to_string(),
                suggestion: "请检查父目录权限或改用 recursive=true".to_string(),
            }
            .to_value()
        })?;

        Ok(IoToolError::success_response(
            "mkdir",
            json!({
                "path": canonical_path,
                "recursive": recursive,
                "created": true
            }),
        ))
    }

    pub fn rename_path(&self, source: String, destination: String) -> Result<Value, Value> {
        let source_path =
            validate_single_path(self.resolver(), &source).map_err(|e| e.to_value())?;
        let destination_path =
            validate_single_path(self.resolver(), &destination).map_err(|e| e.to_value())?;
        let source_obj = Path::new(&source_path);
        let destination_obj = Path::new(&destination_path);

        if !source_obj.exists() {
            return Err(IoToolError::FileNotFound {
                path: source_path,
                suggestion: "请确认源路径存在".to_string(),
            }
            .to_value());
        }

        ensure_path_not_exists(destination_obj).map_err(|e| e.to_value())?;
        ensure_parent_dir_exists(destination_obj).map_err(|e| e.to_value())?;

        fs::rename(source_obj, destination_obj).map_err(|e| {
            IoToolError::IoError {
                message: e.to_string(),
                path: Some(format!("{} -> {}", source_path, destination_path)),
                operation: "rename_path".to_string(),
                suggestion: "请检查目标路径权限或是否存在同名文件".to_string(),
            }
            .to_value()
        })?;

        Ok(IoToolError::success_response(
            "rename_path",
            json!({
                "source": source_path,
                "destination": destination_path,
                "renamed": true
            }),
        ))
    }

    pub fn search_and_replace_multi(
        &self,
        path: String,
        replacements: Vec<PatchChange>,
    ) -> Result<Value, Value> {
        let canonical_path =
            validate_single_path(self.resolver(), &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical_path);
        ensure_file_exists(path_obj).map_err(|e| e.to_value())?;
        ensure_is_file(path_obj).map_err(|e| e.to_value())?;

        let mut content = read_text_file(path_obj).map_err(|e| {
            IoToolError::IoError {
                message: e.to_string(),
                path: Some(canonical_path.clone()),
                operation: "search_and_replace_multi".to_string(),
                suggestion: "请检查文件编码或权限".to_string(),
            }
            .to_value()
        })?;

        let mut applied = Vec::new();
        let mut skipped = Vec::new();

        for replacement in replacements {
            let occurrences = content.matches(&replacement.old_text).count();
            if occurrences == 0 {
                skipped.push(json!({
                    "path": replacement.path,
                    "old_text": replacement.old_text,
                    "reason": "pattern_not_found"
                }));
                continue;
            }

            if replacement.replace_all {
                content = content.replace(&replacement.old_text, &replacement.new_text);
            } else {
                content = content.replacen(&replacement.old_text, &replacement.new_text, 1);
            }

            applied.push(json!({
                "path": replacement.path,
                "old_text": replacement.old_text,
                "new_text": replacement.new_text,
                "replace_all": replacement.replace_all,
                "matched_count": occurrences
            }));
        }

        fs::write(path_obj, &content).map_err(|e| {
            IoToolError::IoError {
                message: e.to_string(),
                path: Some(canonical_path.clone()),
                operation: "search_and_replace_multi".to_string(),
                suggestion: "请检查磁盘空间与写入权限".to_string(),
            }
            .to_value()
        })?;

        Ok(IoToolError::success_response(
            "search_and_replace_multi",
            json!({
                "path": canonical_path,
                "updated": true,
                "applied": applied,
                "skipped": skipped
            }),
        ))
    }

    pub fn apply_patch(&self, path: String, patch: String) -> Result<Value, Value> {
        let canonical_path =
            validate_single_path(self.resolver(), &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical_path);
        ensure_file_exists(path_obj).map_err(|e| e.to_value())?;
        ensure_is_file(path_obj).map_err(|e| e.to_value())?;

        let original = read_text_file(path_obj).map_err(|e| {
            IoToolError::IoError {
                message: e.to_string(),
                path: Some(canonical_path.clone()),
                operation: "apply_patch".to_string(),
                suggestion: "请检查文件编码或权限".to_string(),
            }
            .to_value()
        })?;

        let patched = apply_unified_patch_like(&original, &patch).map_err(|message| {
            IoToolError::Internal {
                message,
                suggestion: "请确认补丁格式为 unified diff".to_string(),
            }
            .to_value()
        })?;

        fs::write(path_obj, &patched).map_err(|e| {
            IoToolError::IoError {
                message: e.to_string(),
                path: Some(canonical_path.clone()),
                operation: "apply_patch".to_string(),
                suggestion: "请检查磁盘空间与写入权限".to_string(),
            }
            .to_value()
        })?;

        Ok(IoToolError::success_response(
            "apply_patch",
            json!({
                "path": canonical_path,
                "patched": true,
                "original_bytes": original.len(),
                "new_bytes": patched.len()
            }),
        ))
    }
}

fn apply_unified_patch_like(content: &str, patch: &str) -> Result<String, String> {
    let mut lines = content
        .lines()
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    let mut offset: isize = 0;
    let mut has_hunk = false;

    let patch_lines = patch.lines().collect::<Vec<_>>();
    let mut i = 0usize;
    while i < patch_lines.len() {
        let line = patch_lines[i];
        if line.starts_with("---") || line.starts_with("+++") || line.starts_with("diff ") {
            i += 1;
            continue;
        }
        if line.starts_with("@@") {
            let (old_start, _) = parse_hunk_header(line)?;
            offset = old_start as isize - 1;
            has_hunk = true;
            i += 1;
            continue;
        }
        if !has_hunk {
            i += 1;
            continue;
        }

        let index = offset.max(0) as usize;
        if let Some(text) = line.strip_prefix(' ') {
            if index >= lines.len() || lines[index] != text {
                return Err(format!("patch context mismatch at line {}", index + 1));
            }
            offset += 1;
        } else if let Some(text) = line.strip_prefix('-') {
            if index >= lines.len() || lines[index] != text {
                return Err(format!("patch delete mismatch at line {}", index + 1));
            }
            lines.remove(index);
        } else if let Some(text) = line.strip_prefix('+') {
            lines.insert(index, text.to_string());
            offset += 1;
        }
        i += 1;
    }

    if !has_hunk {
        return Err("no patch hunk found".to_string());
    }

    Ok(lines.join("\n"))
}

fn parse_hunk_header(header: &str) -> Result<(usize, usize), String> {
    let parts = header.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 3 {
        return Err("invalid hunk header".to_string());
    }

    let old_part = parts[1].trim_start_matches('-');
    let new_part = parts[2].trim_start_matches('+');

    let old_start = old_part
        .split(',')
        .next()
        .unwrap_or_default()
        .parse::<usize>()
        .map_err(|_| "invalid old line number".to_string())?;
    let new_start = new_part
        .split(',')
        .next()
        .unwrap_or_default()
        .parse::<usize>()
        .map_err(|_| "invalid new line number".to_string())?;

    Ok((old_start, new_start))
}
