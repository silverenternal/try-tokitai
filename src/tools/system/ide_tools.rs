use crate::text_encoding::read_text_file;
use crate::tools::io::security::SecurePathResolver;
use crate::tools::io::utils::{ensure_file_exists, ensure_is_file, validate_single_path};
use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokitai::tool;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct IdeTools {
    resolver: SecurePathResolver,
}

impl Default for IdeTools {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct SearchAndReplaceItem {
    pub old_text: String,
    pub new_text: String,
    #[serde(default)]
    pub replace_all: bool,
}

impl IdeTools {
    pub fn new() -> Self {
        Self {
            resolver: SecurePathResolver::new(),
        }
    }

    #[allow(dead_code)]
    pub fn with_resolver(resolver: SecurePathResolver) -> Self {
        Self { resolver }
    }

    fn workspace_root_for(path: &Path) -> PathBuf {
        let mut current = if path.is_dir() {
            path.to_path_buf()
        } else {
            path.parent().unwrap_or(path).to_path_buf()
        };
        loop {
            if current.join("Cargo.toml").exists()
                || current.join("pyproject.toml").exists()
                || current.join("package.json").exists()
                || current.join(".git").exists()
            {
                return current;
            }
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                return path.to_path_buf();
            }
        }
    }

    fn language_from_path(path: &Path) -> &'static str {
        match path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase().as_str() {
            "rs" => "rust",
            "py" => "python",
            "js" | "jsx" => "javascript",
            "ts" | "tsx" => "typescript",
            "json" | "toml" | "md" | "markdown" => "text",
            _ => "text",
        }
    }

    fn is_indexable_file(path: &Path) -> bool {
        matches!(
            path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase()
                .as_str(),
            "rs"
                | "py"
                | "js"
                | "jsx"
                | "ts"
                | "tsx"
                | "java"
                | "go"
                | "c"
                | "cc"
                | "cpp"
                | "cxx"
                | "h"
                | "hpp"
                | "toml"
                | "json"
                | "yaml"
                | "yml"
                | "md"
        )
    }

    fn workspace_root(path: &Path) -> PathBuf {
        if path.is_dir() {
            return path.to_path_buf();
        }
        Self::workspace_root_for(path)
    }

    fn extract_identifier_at(line: &str, column: usize) -> Option<String> {
        if line.is_empty() {
            return None;
        }

        let target = column.saturating_sub(1).min(line.len());
        let mut start = target;
        let mut end = target;

        fn is_ident(ch: char) -> bool {
            ch == '_' || ch.is_alphanumeric()
        }

        while start > 0 {
            let ch = line[..start].chars().next_back()?;
            if !is_ident(ch) {
                break;
            }
            start -= ch.len_utf8();
        }

        while end < line.len() {
            let ch = line[end..].chars().next()?;
            if !is_ident(ch) {
                break;
            }
            end += ch.len_utf8();
        }

        let token = line.get(start..end)?.trim();
        if token.is_empty() {
            None
        } else {
            Some(token.to_string())
        }
    }

    fn symbol_regexes(lang: &str) -> Vec<(&'static str, Regex)> {
        let mut out = Vec::new();
        let mut push = |kind: &'static str, pat: &str| {
            if let Ok(regex) = Regex::new(pat) {
                out.push((kind, regex));
            }
        };

        match lang {
            "rust" => {
                push("function", r"^\s*(?:pub\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)");
                push("struct", r"^\s*(?:pub\s+)?struct\s+([A-Za-z_][A-Za-z0-9_]*)");
                push("enum", r"^\s*(?:pub\s+)?enum\s+([A-Za-z_][A-Za-z0-9_]*)");
                push("trait", r"^\s*(?:pub\s+)?trait\s+([A-Za-z_][A-Za-z0-9_]*)");
                push("type", r"^\s*(?:pub\s+)?type\s+([A-Za-z_][A-Za-z0-9_]*)");
                push("mod", r"^\s*(?:pub\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)");
            }
            "python" => {
                push("function", r"^\s*def\s+([A-Za-z_][A-Za-z0-9_]*)");
                push("class", r"^\s*class\s+([A-Za-z_][A-Za-z0-9_]*)");
            }
            "javascript" | "typescript" => {
                push("function", r"^\s*(?:export\s+)?(?:async\s+)?function\s+([A-Za-z_][A-Za-z0-9_]*)");
                push(
                    "function",
                    r"^\s*(?:export\s+)?(?:const|let|var)\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(?:async\s*)?(?:\(|function\b)",
                );
                push("class", r"^\s*(?:export\s+)?class\s+([A-Za-z_][A-Za-z0-9_]*)");
                push("interface", r"^\s*(?:export\s+)?interface\s+([A-Za-z_][A-Za-z0-9_]*)");
                push("type", r"^\s*(?:export\s+)?type\s+([A-Za-z_][A-Za-z0-9_]*)");
            }
            "java" => {
                push("class", r"^\s*(?:public\s+)?class\s+([A-Za-z_][A-Za-z0-9_]*)");
                push("interface", r"^\s*(?:public\s+)?interface\s+([A-Za-z_][A-Za-z0-9_]*)");
                push(
                    "method",
                    r"^\s*(?:public|private|protected)?\s*(?:static\s+)?[A-Za-z0-9_<>\[\], ?]+\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(",
                );
            }
            "go" | "c" | "cpp" => {
                push("function", r"^\s*(?:func|[A-Za-z_][A-Za-z0-9_<>\[\],\s\*]+)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(");
                push("struct", r"^\s*type\s+([A-Za-z_][A-Za-z0-9_]*)\s+struct\b");
            }
            _ => {}
        }

        out
    }

    fn collect_document_symbols(path: &Path) -> Vec<Value> {
        let content = match read_text_file(path) {
            Ok(content) => content,
            Err(_) => return Vec::new(),
        };
        let regexes = Self::symbol_regexes(Self::language_from_path(path));
        let mut seen = HashSet::new();
        let mut symbols = Vec::new();

        for (line_no, line) in content.lines().enumerate() {
            for (kind, regex) in &regexes {
                if let Some(caps) = regex.captures(line) {
                    if let Some(name) = caps.get(1).map(|m| m.as_str().to_string()) {
                        let key = format!("{}:{}:{}", kind, name, line_no + 1);
                        if !seen.insert(key) {
                            continue;
                        }
                        symbols.push(json!({
                            "name": name,
                            "kind": kind,
                            "line": line_no + 1,
                            "column": caps.get(1).map(|m| m.start() + 1).unwrap_or(1),
                            "signature": line.trim(),
                            "file": path.to_string_lossy().to_string()
                        }));
                    }
                }
            }
        }

        symbols
    }

    fn score_definition_match(candidate: &Value, query_file: &Path, symbol: &str) -> i32 {
        let candidate_file = candidate["file"].as_str().unwrap_or("");
        let candidate_name = candidate["name"].as_str().unwrap_or("");
        let mut score = 0;
        if candidate_name == symbol {
            score += 100;
        } else if candidate_name.eq_ignore_ascii_case(symbol) {
            score += 75;
        } else if candidate_name
            .to_lowercase()
            .contains(&symbol.to_lowercase())
        {
            score += 25;
        }
        if candidate_file == query_file.to_string_lossy() {
            score += 50;
        }
        score
    }

    fn search_definition_candidates(root: &Path, symbol: &str, max_results: usize) -> Vec<Value> {
        let mut results = Vec::new();
        let symbol_lower = symbol.to_lowercase();
        let entries: Box<dyn Iterator<Item = PathBuf>> = if root.is_file() {
            Box::new(std::iter::once(root.to_path_buf()))
        } else {
            Box::new(
                WalkDir::new(root)
                    .into_iter()
                    .filter_map(Result::ok)
                    .map(|entry| entry.path().to_path_buf()),
            )
        };

        for file in entries {
            if results.len() >= max_results {
                break;
            }
            if !file.is_file() || !Self::is_indexable_file(&file) {
                continue;
            }

            for symbol_def in Self::collect_document_symbols(&file) {
                if results.len() >= max_results {
                    break;
                }
                let name = symbol_def["name"].as_str().unwrap_or("");
                if name.eq_ignore_ascii_case(symbol) || name.to_lowercase().contains(&symbol_lower) {
                    results.push(json!({
                        "file": file.to_string_lossy().to_string(),
                        "name": name,
                        "kind": symbol_def["kind"].clone(),
                        "line": symbol_def["line"].clone(),
                        "column": symbol_def["column"].clone(),
                        "signature": symbol_def["signature"].clone(),
                        "match": if name == symbol { "exact" } else { "fuzzy" }
                    }));
                }
            }
        }

        results
    }

    fn count_lines_and_complexity(content: &str) -> (usize, usize, usize, usize) {
        let mut total_lines = 0;
        let mut code_lines = 0;
        let mut function_count = 0;
        let mut cyclomatic = 1usize;

        for line in content.lines() {
            total_lines += 1;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
                continue;
            }
            code_lines += 1;
            let lower = trimmed.to_lowercase();
            if lower.contains(" fn ")
                || lower.starts_with("fn ")
                || lower.contains(" def ")
                || lower.starts_with("def ")
                || lower.contains(" function ")
                || lower.starts_with("function ")
            {
                function_count += 1;
            }
            for keyword in [" if ", " else if ", " for ", " while ", " case ", " catch ", " match ", " and ", " or ", "&&", "||", "?"] {
                if trimmed.contains(keyword) {
                    cyclomatic += 1;
                }
            }
        }

        (total_lines, code_lines, function_count, cyclomatic)
    }

    fn parse_imports(path: &Path) -> Vec<Value> {
        let content = match read_text_file(path) {
            Ok(content) => content,
            Err(_) => return Vec::new(),
        };
        let lang = Self::language_from_path(path);
        let mut imports = Vec::new();

        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            match lang {
                "rust" => {
                    if trimmed.starts_with("use ") || trimmed.starts_with("pub use ") {
                        imports.push(json!({"line": idx + 1, "content": trimmed}));
                    }
                }
                "python" => {
                    if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                        imports.push(json!({"line": idx + 1, "content": trimmed}));
                    }
                }
                "javascript" | "typescript" => {
                    if trimmed.starts_with("import ")
                        || trimmed.starts_with("export ")
                        || trimmed.contains("require(")
                    {
                        imports.push(json!({"line": idx + 1, "content": trimmed}));
                    }
                }
                "java" => {
                    if trimmed.starts_with("import ") {
                        imports.push(json!({"line": idx + 1, "content": trimmed}));
                    }
                }
                "go" => {
                    if trimmed.starts_with("import ") || trimmed == "(" || trimmed == ")" {
                        imports.push(json!({"line": idx + 1, "content": trimmed}));
                    }
                }
                _ => {}
            }
        }

        imports
    }

    fn scan_workspace_text(root: &Path, pattern: &Regex, max_results: usize) -> Vec<Value> {
        let mut results = Vec::new();
        let entries: Box<dyn Iterator<Item = PathBuf>> = if root.is_file() {
            Box::new(std::iter::once(root.to_path_buf()))
        } else {
            Box::new(
                WalkDir::new(root)
                    .into_iter()
                    .filter_map(Result::ok)
                    .map(|entry| entry.path().to_path_buf()),
            )
        };

        for file in entries {
            if results.len() >= max_results {
                break;
            }
            if !file.is_file() || !Self::is_indexable_file(&file) {
                continue;
            }
            let content = match read_text_file(&file) {
                Ok(content) => content,
                Err(_) => continue,
            };
            for (idx, line) in content.lines().enumerate() {
                if results.len() >= max_results {
                    break;
                }
                if pattern.is_match(line) {
                    results.push(json!({
                        "file": file.to_string_lossy().to_string(),
                        "line": idx + 1,
                        "content": line.trim()
                    }));
                }
            }
        }

        results
    }

    fn infer_symbol_calls(content: &str, symbol: &str) -> Vec<usize> {
        let mut lines = Vec::new();
        let call_patterns = [
            format!("{}(", symbol),
            format!(".{}(", symbol),
            format!(" {}(", symbol),
        ];
        for (idx, line) in content.lines().enumerate() {
            if call_patterns.iter().any(|p| line.contains(p)) {
                lines.push(idx + 1);
            }
        }
        lines
    }

    fn build_codelens(path: &Path, content: &str) -> Vec<Value> {
        let lang = Self::language_from_path(path);
        let symbols = Self::collect_document_symbols(path);
        let mut lenses = Vec::new();

        for symbol in symbols {
            let name = symbol["name"].as_str().unwrap_or("").to_string();
            let line = symbol["line"].as_u64().unwrap_or(1) as usize;
            let occurrences = content
                .lines()
                .filter(|l| l.contains(&name))
                .count();
            let kind = symbol["kind"].as_str().unwrap_or("symbol");
            lenses.push(json!({
                "line": line,
                "kind": kind,
                "title": format!("{} occurrences: {}", name, occurrences),
                "language": lang,
                "name": name,
                "range": {
                    "start_line": line,
                    "end_line": line
                }
            }));
        }

        lenses
    }

    fn collect_workspace_files(root: &Path) -> Vec<PathBuf> {
        if root.is_file() {
            return vec![root.to_path_buf()];
        }
        WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
            .map(|entry| entry.path().to_path_buf())
            .filter(|p| p.is_file() && Self::is_indexable_file(p))
            .collect()
    }

    fn snapshot_payload(&self, root: &Path) -> Value {
        let mut files = Vec::new();
        for file in Self::collect_workspace_files(root) {
            let content = match read_text_file(&file) {
                Ok(content) => content,
                Err(_) => continue,
            };
            let (total_lines, code_lines, function_count, cyclomatic) =
                Self::count_lines_and_complexity(&content);
            files.push(json!({
                "file": file.to_string_lossy().to_string(),
                "language": Self::language_from_path(&file),
                "size_bytes": content.len(),
                "total_lines": total_lines,
                "code_lines": code_lines,
                "function_count": function_count,
                "cyclomatic_complexity_estimate": cyclomatic,
                "imports": Self::parse_imports(&file),
                "symbols": Self::collect_document_symbols(&file)
            }));
        }

        json!({
            "root": root.to_string_lossy().to_string(),
            "files": files,
            "file_count": files.len()
        })
    }
}

#[tool]
impl IdeTools {
    pub fn diagnostics(&self, path: String) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        ensure_file_exists(path_obj).map_err(|e| e.to_value())?;
        ensure_is_file(path_obj).map_err(|e| e.to_value())?;

        let lang = Self::language_from_path(path_obj);
        let mut checks = Vec::new();

        match lang {
            "rust" => {
                let root = Self::workspace_root_for(path_obj);
                let cargo = Command::new("cargo")
                    .args(["check", "--message-format", "short"])
                    .current_dir(root)
                    .output();
                match cargo {
                    Ok(output) => {
                        checks.push(json!({
                            "tool": "cargo check",
                            "success": output.status.success(),
                            "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
                            "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
                        }));
                    }
                    Err(err) => {
                        checks.push(json!({
                            "tool": "cargo check",
                            "success": false,
                            "stderr": err.to_string()
                        }));
                    }
                }
            }
            "python" => {
                let result = Command::new("python")
                    .args(["-m", "py_compile", &canonical])
                    .output();
                match result {
                    Ok(output) => {
                        checks.push(json!({
                            "tool": "python -m py_compile",
                            "success": output.status.success(),
                            "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
                        }));
                    }
                    Err(err) => {
                        checks.push(json!({
                            "tool": "python -m py_compile",
                            "success": false,
                            "stderr": err.to_string()
                        }));
                    }
                }
            }
            "javascript" | "typescript" => {
                let node = Command::new("node")
                    .args(["--check", &canonical])
                    .output();
                match node {
                    Ok(output) => {
                        checks.push(json!({
                            "tool": "node --check",
                            "success": output.status.success(),
                            "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
                        }));
                    }
                    Err(err) => {
                        checks.push(json!({
                            "tool": "node --check",
                            "success": false,
                            "stderr": err.to_string()
                        }));
                    }
                }
            }
            _ => {
                checks.push(json!({
                    "tool": "file check",
                    "success": true,
                    "message": "no language-specific diagnostics available"
                }));
            }
        }

        let has_errors = checks.iter().any(|check| !check["success"].as_bool().unwrap_or(false));
        Ok(json!({
            "status": "success",
            "operation": "diagnostics",
            "data": {
                "path": canonical,
                "language": lang,
                "has_errors": has_errors,
                "checks": checks
            }
        }))
    }

    pub fn symbol_search(
        &self,
        path: String,
        symbol: String,
        max_results: Option<usize>,
    ) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        let max_results = max_results.unwrap_or(50).clamp(1, 500);
        let symbol_re = Regex::new(&format!(r"\b{}\b", regex::escape(&symbol)))
            .map_err(|e| tool_error(e.to_string()))?;

        let mut results = Vec::new();
        let walker = if path_obj.is_dir() {
            WalkDir::new(path_obj).into_iter()
        } else {
            WalkDir::new(path_obj.parent().unwrap_or(path_obj)).max_depth(1).into_iter()
        };

        for entry in walker.filter_map(Result::ok) {
            if results.len() >= max_results {
                break;
            }
            let file = entry.path();
            if !file.is_file() {
                continue;
            }
            let text = match read_text_file(file) {
                Ok(text) => text,
                Err(_) => continue,
            };
            for (idx, line) in text.lines().enumerate() {
                if results.len() >= max_results {
                    break;
                }
                if symbol_re.is_match(line) {
                    results.push(json!({
                        "file": file.to_string_lossy().to_string(),
                        "line": idx + 1,
                        "content": line.trim()
                    }));
                }
            }
        }

        Ok(json!({
            "status": "success",
            "operation": "symbol_search",
            "data": {
                "path": canonical,
                "symbol": symbol,
                "count": results.len(),
                "results": results
            }
        }))
    }

    pub fn references_search(
        &self,
        path: String,
        symbol: String,
        max_results: Option<usize>,
    ) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        let max_results = max_results.unwrap_or(100).clamp(1, 1000);
        let symbol_re = Regex::new(&regex::escape(&symbol))
            .map_err(|e| tool_error(e.to_string()))?;
        let mut results = Vec::new();
        let root = if path_obj.is_dir() {
            path_obj.to_path_buf()
        } else {
            path_obj.parent().unwrap_or(path_obj).to_path_buf()
        };

        for entry in WalkDir::new(&root).into_iter().filter_map(Result::ok) {
            if results.len() >= max_results {
                break;
            }
            let file = entry.path();
            if !file.is_file() {
                continue;
            }
            let text = match read_text_file(file) {
                Ok(text) => text,
                Err(_) => continue,
            };
            for (idx, line) in text.lines().enumerate() {
                if results.len() >= max_results {
                    break;
                }
                if symbol_re.is_match(line) {
                    results.push(json!({
                        "file": file.to_string_lossy().to_string(),
                        "line": idx + 1,
                        "content": line.trim()
                    }));
                }
            }
        }

        Ok(json!({
            "status": "success",
            "operation": "references_search",
            "data": {
                "path": canonical,
                "symbol": symbol,
                "count": results.len(),
                "results": results
            }
        }))
    }

    pub fn format_file(&self, path: String) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        ensure_file_exists(path_obj).map_err(|e| e.to_value())?;
        ensure_is_file(path_obj).map_err(|e| e.to_value())?;

        let lang = Self::language_from_path(path_obj);
        let status = match lang {
            "rust" => Command::new("rustfmt").arg(&canonical).status(),
            "python" => Command::new("python").args(["-m", "black", &canonical]).status(),
            "javascript" | "typescript" => Command::new("prettier").args(["--write", &canonical]).status(),
            _ => return Err(tool_error("no formatter available for this file type".to_string())),
        };

        match status {
            Ok(exit) if exit.success() => Ok(json!({
                "status": "success",
                "operation": "format_file",
                "data": {
                    "path": canonical,
                    "language": lang,
                    "formatted": true
                }
            })),
            Ok(exit) => Err(tool_error(format!("formatter exited with {}", exit))),
            Err(err) => Err(tool_error(err.to_string())),
        }
    }

    pub fn test_target(&self, target: String, filter: Option<String>) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &target).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        let root = Self::workspace_root_for(path_obj);
        let mut cmd = Command::new("cargo");
        cmd.current_dir(root).arg("test").arg("--quiet");

        if let Some(filter) = filter {
            if !filter.trim().is_empty() {
                cmd.arg(filter.trim());
            }
        } else if path_obj.is_file() {
            if let Some(stem) = path_obj.file_stem().and_then(|s| s.to_str()) {
                cmd.arg(stem);
            }
        }

        let output = cmd.output().map_err(|e| tool_error(format!("failed to run cargo test: {}", e)))?;

        Ok(json!({
            "status": if output.status.success() { "success" } else { "error" },
            "operation": "test_target",
            "data": {
                "target": canonical,
                "exit_code": output.status.code().unwrap_or(-1),
                "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
                "stderr": String::from_utf8_lossy(&output.stderr).to_string()
            }
        }))
    }

    pub fn document_symbols(&self, path: String) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        ensure_file_exists(path_obj).map_err(|e| e.to_value())?;
        ensure_is_file(path_obj).map_err(|e| e.to_value())?;
        let symbols = Self::collect_document_symbols(path_obj);

        Ok(json!({
            "status": "success",
            "operation": "document_symbols",
            "data": {
                "path": canonical,
                "count": symbols.len(),
                "symbols": symbols
            }
        }))
    }

    pub fn workspace_symbols(
        &self,
        path: String,
        query: String,
        max_results: Option<usize>,
    ) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        let root = Self::workspace_root(path_obj);
        let max_results = max_results.unwrap_or(100).clamp(1, 500);
        let mut symbols = Self::search_definition_candidates(&root, &query, max_results * 2);
        symbols.sort_by(|a, b| {
            let sa = Self::score_definition_match(a, path_obj, &query);
            let sb = Self::score_definition_match(b, path_obj, &query);
            sb.cmp(&sa)
        });
        symbols.truncate(max_results);

        Ok(json!({
            "status": "success",
            "operation": "workspace_symbols",
            "data": {
                "root": root.to_string_lossy().to_string(),
                "query": query,
                "count": symbols.len(),
                "symbols": symbols
            }
        }))
    }

    pub fn go_to_definition(
        &self,
        path: String,
        symbol: String,
        max_results: Option<usize>,
    ) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        let root = Self::workspace_root(path_obj);
        let max_results = max_results.unwrap_or(20).clamp(1, 100);
        let mut definitions = Self::search_definition_candidates(&root, &symbol, max_results * 2);
        definitions.sort_by(|a, b| {
            let sa = Self::score_definition_match(a, path_obj, &symbol);
            let sb = Self::score_definition_match(b, path_obj, &symbol);
            sb.cmp(&sa)
        });
        definitions.truncate(max_results);

        Ok(json!({
            "status": "success",
            "operation": "go_to_definition",
            "data": {
                "query_file": canonical,
                "symbol": symbol,
                "count": definitions.len(),
                "definitions": definitions
            }
        }))
    }

    pub fn find_implementations(
        &self,
        path: String,
        symbol: String,
        max_results: Option<usize>,
    ) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        let root = Self::workspace_root(path_obj);
        let max_results = max_results.unwrap_or(50).clamp(1, 200);
        let symbol_re = Regex::new(&regex::escape(&symbol)).map_err(|e| tool_error(e.to_string()))?;
        let mut results = Vec::new();

        for entry in WalkDir::new(&root).into_iter().filter_map(Result::ok) {
            if results.len() >= max_results {
                break;
            }
            let file = entry.path();
            if !file.is_file() || !Self::is_indexable_file(file) {
                continue;
            }
            let content = match read_text_file(file) {
                Ok(content) => content,
                Err(_) => continue,
            };
            let lang = Self::language_from_path(file);
            for (idx, line) in content.lines().enumerate() {
                if results.len() >= max_results {
                    break;
                }
                let matched = match lang {
                    "rust" => {
                        line.contains(&format!("impl {}", symbol))
                            || (line.contains("impl<") && symbol_re.is_match(line))
                            || line.contains(&format!(".{}", symbol))
                    }
                    "python" => {
                        line.contains(&format!("class {}", symbol))
                            || line.contains(&format!("def {}", symbol))
                    }
                    "javascript" | "typescript" => {
                        line.contains(&format!("class {}", symbol))
                            || line.contains(&format!("extends {}", symbol))
                            || line.contains(&format!("implements {}", symbol))
                            || line.contains(&format!("prototype.{}", symbol))
                    }
                    "java" => {
                        line.contains(&format!("extends {}", symbol))
                            || line.contains(&format!("implements {}", symbol))
                            || line.contains(&format!("class {}", symbol))
                    }
                    _ => symbol_re.is_match(line),
                };

                if matched {
                    results.push(json!({
                        "file": file.to_string_lossy().to_string(),
                        "line": idx + 1,
                        "content": line.trim(),
                        "language": lang
                    }));
                }
            }
        }

        Ok(json!({
            "status": "success",
            "operation": "find_implementations",
            "data": {
                "root": root.to_string_lossy().to_string(),
                "symbol": symbol,
                "count": results.len(),
                "implementations": results
            }
        }))
    }

    pub fn hover(
        &self,
        path: String,
        line: usize,
        column: Option<usize>,
    ) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        ensure_file_exists(path_obj).map_err(|e| e.to_value())?;
        ensure_is_file(path_obj).map_err(|e| e.to_value())?;
        let content = read_text_file(path_obj).map_err(|e| tool_error(e.to_string()))?;
        let line_text = content
            .lines()
            .nth(line.saturating_sub(1))
            .ok_or_else(|| tool_error("line out of range".to_string()))?;
        let token = column.and_then(|col| Self::extract_identifier_at(line_text, col));
        let definitions = token
            .as_ref()
            .map(|symbol| Self::search_definition_candidates(&Self::workspace_root(path_obj), symbol, 5))
            .unwrap_or_default();

        Ok(json!({
            "status": "success",
            "operation": "hover",
            "data": {
                "path": canonical,
                "line": line,
                "column": column,
                "language": Self::language_from_path(path_obj),
                "token": token,
                "content": line_text.trim(),
                "definitions": definitions
            }
        }))
    }

    pub fn signature_help(
        &self,
        path: String,
        line: usize,
        column: Option<usize>,
    ) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        ensure_file_exists(path_obj).map_err(|e| e.to_value())?;
        ensure_is_file(path_obj).map_err(|e| e.to_value())?;
        let content = read_text_file(path_obj).map_err(|e| tool_error(e.to_string()))?;
        let line_text = content
            .lines()
            .nth(line.saturating_sub(1))
            .ok_or_else(|| tool_error("line out of range".to_string()))?;
        let token = column.and_then(|col| Self::extract_identifier_at(line_text, col));
        let mut signatures = token
            .as_ref()
            .map(|symbol| Self::search_definition_candidates(&Self::workspace_root(path_obj), symbol, 10))
            .unwrap_or_default();
        signatures.sort_by(|a, b| {
            let symbol = token.as_deref().unwrap_or("");
            let sa = Self::score_definition_match(a, path_obj, symbol);
            let sb = Self::score_definition_match(b, path_obj, symbol);
            sb.cmp(&sa)
        });

        Ok(json!({
            "status": "success",
            "operation": "signature_help",
            "data": {
                "path": canonical,
                "line": line,
                "column": column,
                "token": token,
                "signatures": signatures
            }
        }))
    }

    pub fn rename_symbol(
        &self,
        path: String,
        old_name: String,
        new_name: String,
        max_results: Option<usize>,
    ) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        let root = Self::workspace_root(path_obj);
        let max_results = max_results.unwrap_or(200).clamp(1, 1000);

        if old_name.trim().is_empty() || new_name.trim().is_empty() {
            return Err(tool_error("old_name and new_name are required".to_string()));
        }
        if old_name == new_name {
            return Err(tool_error("old_name and new_name must differ".to_string()));
        }

        let token_re = Regex::new(&format!(r"\b{}\b", regex::escape(&old_name)))
            .map_err(|e| tool_error(e.to_string()))?;
        let mut changes = Vec::new();

        for entry in WalkDir::new(&root).into_iter().filter_map(Result::ok) {
            if changes.len() >= max_results {
                break;
            }
            let file = entry.path();
            if !file.is_file() || !Self::is_indexable_file(file) {
                continue;
            }
            let content = match read_text_file(file) {
                Ok(content) => content,
                Err(_) => continue,
            };
            let mut file_changes = Vec::new();
            for (idx, line) in content.lines().enumerate() {
                if token_re.is_match(line) {
                    file_changes.push(json!({
                        "line": idx + 1,
                        "before": line.trim(),
                        "after": token_re.replace_all(line, new_name.as_str()).to_string().trim().to_string()
                    }));
                }
            }
            if !file_changes.is_empty() {
                changes.push(json!({
                    "file": file.to_string_lossy().to_string(),
                    "change_count": file_changes.len(),
                    "changes": file_changes
                }));
            }
        }

        Ok(json!({
            "status": "success",
            "operation": "rename_symbol",
            "data": {
                "path": canonical,
                "workspace_root": root.to_string_lossy().to_string(),
                "old_name": old_name,
                "new_name": new_name,
                "count": changes.len(),
                "changes": changes,
                "note": "This is a preview-only rename plan. Apply with edit_file or apply_patch after review."
            }
        }))
    }

    pub fn file_complexity(&self, path: String) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        ensure_file_exists(path_obj).map_err(|e| e.to_value())?;
        ensure_is_file(path_obj).map_err(|e| e.to_value())?;
        let content = read_text_file(path_obj).map_err(|e| tool_error(e.to_string()))?;
        let (total_lines, code_lines, function_count, cyclomatic) = Self::count_lines_and_complexity(&content);

        Ok(json!({
            "status": "success",
            "operation": "file_complexity",
            "data": {
                "path": canonical,
                "language": Self::language_from_path(path_obj),
                "total_lines": total_lines,
                "code_lines": code_lines,
                "function_count": function_count,
                "cyclomatic_complexity_estimate": cyclomatic
            }
        }))
    }

    pub fn import_map(&self, path: String) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        let root = Self::workspace_root(path_obj);
        let mut files = Vec::new();
        if root.is_file() {
            files.push(root.clone());
        } else {
            for entry in WalkDir::new(&root).into_iter().filter_map(Result::ok) {
                let file = entry.path();
                if file.is_file() && Self::is_indexable_file(file) {
                    files.push(file.to_path_buf());
                }
            }
        }

        let mut report = Vec::new();
        for file in files {
            let imports = Self::parse_imports(&file);
            if !imports.is_empty() {
                report.push(json!({
                    "file": file.to_string_lossy().to_string(),
                    "language": Self::language_from_path(&file),
                    "import_count": imports.len(),
                    "imports": imports
                }));
            }
        }

        Ok(json!({
            "status": "success",
            "operation": "import_map",
            "data": {
                "root": root.to_string_lossy().to_string(),
                "count": report.len(),
                "files": report
            }
        }))
    }

    pub fn api_surface(&self, path: String) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        let root = Self::workspace_root(path_obj);
        let mut files = Vec::new();
        if root.is_file() {
            files.push(root.clone());
        } else {
            for entry in WalkDir::new(&root).into_iter().filter_map(Result::ok) {
                let file = entry.path();
                if file.is_file() && Self::is_indexable_file(file) {
                    files.push(file.to_path_buf());
                }
            }
        }

        let mut documents = Vec::new();
        for file in files {
            let symbols = Self::collect_document_symbols(&file);
            if !symbols.is_empty() {
                documents.push(json!({
                    "file": file.to_string_lossy().to_string(),
                    "language": Self::language_from_path(&file),
                    "count": symbols.len(),
                    "symbols": symbols
                }));
            }
        }

        Ok(json!({
            "status": "success",
            "operation": "api_surface",
            "data": {
                "root": root.to_string_lossy().to_string(),
                "count": documents.len(),
                "documents": documents
            }
        }))
    }

    pub fn project_dependency_graph(&self, path: String) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        let root = Self::workspace_root(path_obj);
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for entry in WalkDir::new(&root).into_iter().filter_map(Result::ok) {
            let file = entry.path();
            if !file.is_file() {
                continue;
            }
            let imports = Self::parse_imports(file);
            if imports.is_empty() {
                continue;
            }

            let file_str = file.to_string_lossy().to_string();
            nodes.push(json!({
                "file": file_str,
                "language": Self::language_from_path(file),
                "imports": imports.len()
            }));

            for import in imports {
                edges.push(json!({
                    "from": file.to_string_lossy().to_string(),
                    "to": import["content"].clone(),
                    "line": import["line"].clone()
                }));
            }
        }

        Ok(json!({
            "status": "success",
            "operation": "project_dependency_graph",
            "data": {
                "root": root.to_string_lossy().to_string(),
                "node_count": nodes.len(),
                "edge_count": edges.len(),
                "nodes": nodes,
                "edges": edges
            }
        }))
    }

    pub fn search_workspace_text(
        &self,
        path: String,
        pattern: String,
        max_results: Option<usize>,
    ) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        let root = Self::workspace_root(path_obj);
        let max_results = max_results.unwrap_or(200).clamp(1, 2000);
        let regex = Regex::new(&pattern).map_err(|e| tool_error(e.to_string()))?;
        let matches = Self::scan_workspace_text(&root, &regex, max_results);

        Ok(json!({
            "status": "success",
            "operation": "search_workspace_text",
            "data": {
                "root": root.to_string_lossy().to_string(),
                "pattern": pattern,
                "count": matches.len(),
                "matches": matches
            }
        }))
    }

    pub fn goto_implementation(
        &self,
        path: String,
        symbol: String,
        max_results: Option<usize>,
    ) -> Result<Value, Value> {
        self.find_implementations(path, symbol, max_results)
    }

    pub fn call_hierarchy(
        &self,
        path: String,
        symbol: String,
        max_results: Option<usize>,
    ) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        let root = Self::workspace_root(path_obj);
        let max_results = max_results.unwrap_or(100).clamp(1, 1000);
        let mut callers = Vec::new();
        let mut callees = Vec::new();
        let mut defs = Self::search_definition_candidates(&root, &symbol, 10);
        defs.sort_by(|a, b| {
            let sa = Self::score_definition_match(a, path_obj, &symbol);
            let sb = Self::score_definition_match(b, path_obj, &symbol);
            sb.cmp(&sa)
        });

        let call_re = Regex::new(&format!(r"\b{}\s*\(", regex::escape(&symbol)))
            .map_err(|e| tool_error(e.to_string()))?;
        let definitions = defs;
        let entries: Vec<PathBuf> = if root.is_file() {
            vec![root.clone()]
        } else {
            WalkDir::new(&root)
                .into_iter()
                .filter_map(Result::ok)
                .map(|entry| entry.path().to_path_buf())
                .collect()
        };

        for file in entries {
            if callers.len() >= max_results || callees.len() >= max_results {
                break;
            }
            if !file.is_file() || !Self::is_indexable_file(&file) {
                continue;
            }
            let content = match read_text_file(&file) {
                Ok(content) => content,
                Err(_) => continue,
            };

            for (idx, line) in content.lines().enumerate() {
                if callers.len() >= max_results {
                    break;
                }
                if call_re.is_match(line) {
                    callers.push(json!({
                        "file": file.to_string_lossy().to_string(),
                        "line": idx + 1,
                        "content": line.trim()
                    }));
                }
            }

            for symbol_def in &definitions {
                if callees.len() >= max_results {
                    break;
                }
                let def_name = symbol_def["name"].as_str().unwrap_or("");
                if def_name == symbol || def_name.eq_ignore_ascii_case(&symbol) {
                    let content = match read_text_file(&file) {
                        Ok(content) => content,
                        Err(_) => continue,
                    };
                    let call_lines = Self::infer_symbol_calls(&content, &symbol);
                    for line_no in call_lines {
                        if callees.len() >= max_results {
                            break;
                        }
                        if let Some(line_text) = content.lines().nth(line_no.saturating_sub(1)) {
                            callees.push(json!({
                                "file": file.to_string_lossy().to_string(),
                                "line": line_no,
                                "content": line_text.trim()
                            }));
                        }
                    }
                }
            }
        }

        Ok(json!({
            "status": "success",
            "operation": "call_hierarchy",
            "data": {
                "path": canonical,
                "symbol": symbol,
                "definitions": definitions,
                "callers": callers,
                "callees": callees
            }
        }))
    }

    pub fn change_hierarchy(
        &self,
        path: String,
        symbol: String,
        max_results: Option<usize>,
    ) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        let root = Self::workspace_root(path_obj);
        let max_results = max_results.unwrap_or(100).clamp(1, 1000);
        let mut changes = Vec::new();
        let token_re = Regex::new(&format!(r"\b{}\b", regex::escape(&symbol)))
            .map_err(|e| tool_error(e.to_string()))?;

        for entry in WalkDir::new(&root).into_iter().filter_map(Result::ok) {
            if changes.len() >= max_results {
                break;
            }
            let file = entry.path();
            if !file.is_file() || !Self::is_indexable_file(file) {
                continue;
            }
            let content = match read_text_file(file) {
                Ok(content) => content,
                Err(_) => continue,
            };

            for (idx, line) in content.lines().enumerate() {
                if changes.len() >= max_results {
                    break;
                }
                if token_re.is_match(line) {
                    let old_count = line.matches(&symbol).count();
                    changes.push(json!({
                        "file": file.to_string_lossy().to_string(),
                        "line": idx + 1,
                        "content": line.trim(),
                        "symbol": symbol,
                        "occurrences": old_count
                    }));
                }
            }
        }

        Ok(json!({
            "status": "success",
            "operation": "change_hierarchy",
            "data": {
                "path": canonical,
                "root": root.to_string_lossy().to_string(),
                "symbol": symbol,
                "count": changes.len(),
                "changes": changes
            }
        }))
    }

    pub fn code_lens(
        &self,
        path: String,
        max_results: Option<usize>,
    ) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        ensure_file_exists(path_obj).map_err(|e| e.to_value())?;
        ensure_is_file(path_obj).map_err(|e| e.to_value())?;
        let content = read_text_file(path_obj).map_err(|e| tool_error(e.to_string()))?;
        let mut lenses = Self::build_codelens(path_obj, &content);
        let max_results = max_results.unwrap_or(100).clamp(1, 500);
        if lenses.len() > max_results {
            lenses.truncate(max_results);
        }

        Ok(json!({
            "status": "success",
            "operation": "code_lens",
            "data": {
                "path": canonical,
                "count": lenses.len(),
                "lenses": lenses
            }
        }))
    }

    pub fn diagnostic_summary(
        &self,
        path: String,
        max_results: Option<usize>,
    ) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        let root = Self::workspace_root(path_obj);
        let max_results = max_results.unwrap_or(200).clamp(1, 2000);
        let mut items = Vec::new();

        for file in Self::collect_workspace_files(&root) {
            if items.len() >= max_results {
                break;
            }
            let content = match read_text_file(&file) {
                Ok(content) => content,
                Err(_) => continue,
            };
            let (total_lines, code_lines, function_count, cyclomatic) =
                Self::count_lines_and_complexity(&content);
            let diag = self.diagnostics(file.to_string_lossy().to_string())?;
            items.push(json!({
                "file": file.to_string_lossy().to_string(),
                "language": Self::language_from_path(&file),
                "total_lines": total_lines,
                "code_lines": code_lines,
                "function_count": function_count,
                "cyclomatic_complexity_estimate": cyclomatic,
                "diagnostics": diag["data"].clone()
            }));
        }

        Ok(json!({
            "status": "success",
            "operation": "diagnostic_summary",
            "data": {
                "root": root.to_string_lossy().to_string(),
                "count": items.len(),
                "files": items
            }
        }))
    }

    pub fn dependency_hotspots(
        &self,
        path: String,
        max_results: Option<usize>,
    ) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let path_obj = Path::new(&canonical);
        let root = Self::workspace_root(path_obj);
        let max_results = max_results.unwrap_or(50).clamp(1, 500);
        let mut hotspots = Vec::new();

        for file in Self::collect_workspace_files(&root) {
            let imports = Self::parse_imports(&file);
            let symbols = Self::collect_document_symbols(&file);
            let content = match read_text_file(&file) {
                Ok(content) => content,
                Err(_) => continue,
            };
            let (_, code_lines, function_count, cyclomatic) = Self::count_lines_and_complexity(&content);
            let score = imports.len() * 3 + symbols.len() * 2 + function_count + cyclomatic + code_lines / 20;

            hotspots.push(json!({
                "file": file.to_string_lossy().to_string(),
                "language": Self::language_from_path(&file),
                "imports": imports.len(),
                "symbols": symbols.len(),
                "functions": function_count,
                "cyclomatic_complexity_estimate": cyclomatic,
                "score": score
            }));
        }

        hotspots.sort_by(|a, b| {
            b["score"].as_u64().unwrap_or(0).cmp(&a["score"].as_u64().unwrap_or(0))
        });
        hotspots.truncate(max_results);

        Ok(json!({
            "status": "success",
            "operation": "dependency_hotspots",
            "data": {
                "root": root.to_string_lossy().to_string(),
                "count": hotspots.len(),
                "hotspots": hotspots
            }
        }))
    }

    pub fn test_impact_analysis(
        &self,
        path: String,
        changed_path: String,
        max_results: Option<usize>,
    ) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let changed = validate_single_path(&self.resolver, &changed_path).map_err(|e| e.to_value())?;
        let root = Self::workspace_root(Path::new(&canonical));
        let max_results = max_results.unwrap_or(50).clamp(1, 500);
        let changed_name = Path::new(&changed)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let mut impacts = Vec::new();
        let test_name_re = Regex::new(r"(?i)test|spec|integration|integration_test|e2e|bench")
            .map_err(|e| tool_error(e.to_string()))?;

        for file in Self::collect_workspace_files(&root) {
            if impacts.len() >= max_results {
                break;
            }
            let content = match read_text_file(&file) {
                Ok(content) => content,
                Err(_) => continue,
            };
            let mut relevance = 0usize;
            if file
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.contains(&changed_name))
                .unwrap_or(false)
            {
                relevance += 5;
            }
            if content.contains(&changed_name) {
                relevance += 3;
            }
            if test_name_re.is_match(&file.to_string_lossy()) {
                relevance += 2;
            }
            if relevance > 0 {
                impacts.push(json!({
                    "file": file.to_string_lossy().to_string(),
                    "language": Self::language_from_path(&file),
                    "relevance": relevance,
                    "reason": if file.file_stem().and_then(|s| s.to_str()).map(|s| s.contains(&changed_name)).unwrap_or(false) {
                        "filename match"
                    } else if content.contains(&changed_name) {
                        "text reference match"
                    } else {
                        "test-like file"
                    }
                }));
            }
        }

        impacts.sort_by(|a, b| {
            b["relevance"].as_u64().unwrap_or(0).cmp(&a["relevance"].as_u64().unwrap_or(0))
        });
        impacts.truncate(max_results);

        Ok(json!({
            "status": "success",
            "operation": "test_impact_analysis",
            "data": {
                "root": root.to_string_lossy().to_string(),
                "changed_path": changed,
                "count": impacts.len(),
                "impacts": impacts
            }
        }))
    }

    pub fn save_analysis_snapshot(
        &self,
        path: String,
        snapshot_path: String,
    ) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let snapshot_canonical =
            validate_single_path(&self.resolver, &snapshot_path).map_err(|e| e.to_value())?;
        let root = Self::workspace_root(Path::new(&canonical));
        let snapshot = self.snapshot_payload(&root);
        let snapshot_string = serde_json::to_string_pretty(&snapshot).map_err(|e| tool_error(e.to_string()))?;
        fs::write(&snapshot_canonical, snapshot_string).map_err(|e| tool_error(e.to_string()))?;

        Ok(json!({
            "status": "success",
            "operation": "save_analysis_snapshot",
            "data": {
                "root": root.to_string_lossy().to_string(),
                "snapshot_path": snapshot_canonical,
                "file_count": snapshot["file_count"].clone()
            }
        }))
    }

    pub fn compare_snapshots(
        &self,
        before_path: String,
        after_path: String,
    ) -> Result<Value, Value> {
        let before = validate_single_path(&self.resolver, &before_path).map_err(|e| e.to_value())?;
        let after = validate_single_path(&self.resolver, &after_path).map_err(|e| e.to_value())?;
        let before_text = fs::read_to_string(&before).map_err(|e| tool_error(e.to_string()))?;
        let after_text = fs::read_to_string(&after).map_err(|e| tool_error(e.to_string()))?;
        let before_json: Value = serde_json::from_str(&before_text).map_err(|e| tool_error(e.to_string()))?;
        let after_json: Value = serde_json::from_str(&after_text).map_err(|e| tool_error(e.to_string()))?;

        let before_files = before_json["files"].as_array().cloned().unwrap_or_default();
        let after_files = after_json["files"].as_array().cloned().unwrap_or_default();
        let before_map: std::collections::HashMap<_, _> = before_files
            .into_iter()
            .filter_map(|item| {
                let path = item["file"].as_str()?.to_string();
                Some((path, item))
            })
            .collect();
        let after_map: std::collections::HashMap<_, _> = after_files
            .into_iter()
            .filter_map(|item| {
                let path = item["file"].as_str()?.to_string();
                Some((path, item))
            })
            .collect();

        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut changed = Vec::new();

        for path in after_map.keys() {
            if !before_map.contains_key(path) {
                added.push(path.clone());
            }
        }
        for path in before_map.keys() {
            if !after_map.contains_key(path) {
                removed.push(path.clone());
            }
        }
        for (path, after_item) in &after_map {
            if let Some(before_item) = before_map.get(path) {
                if before_item != after_item {
                    changed.push(json!({
                        "file": path,
                        "before": before_item,
                        "after": after_item
                    }));
                }
            }
        }

        Ok(json!({
            "status": "success",
            "operation": "compare_snapshots",
            "data": {
                "before": before,
                "after": after,
                "added": added,
                "removed": removed,
                "changed": changed,
                "added_count": added.len(),
                "removed_count": removed.len(),
                "changed_count": changed.len()
            }
        }))
    }

    pub fn workspace_risk_report(
        &self,
        path: String,
        max_results: Option<usize>,
    ) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let root = Self::workspace_root(Path::new(&canonical));
        let max_results = max_results.unwrap_or(50).clamp(1, 500);
        let diagnostics = self.diagnostic_summary(canonical.clone(), Some(max_results))?;
        let hotspots = self.dependency_hotspots(canonical.clone(), Some(max_results))?;
        let impacts = Self::collect_workspace_files(&root)
            .into_iter()
            .take(max_results)
            .map(|file| {
                let content = read_text_file(&file).unwrap_or_default();
                let (_t, code_lines, function_count, cyclomatic) = Self::count_lines_and_complexity(&content);
                json!({
                    "file": file.to_string_lossy().to_string(),
                    "language": Self::language_from_path(&file),
                    "code_lines": code_lines,
                    "function_count": function_count,
                    "cyclomatic_complexity_estimate": cyclomatic,
                    "risk_score": code_lines + function_count + cyclomatic
                })
            })
            .collect::<Vec<_>>();

        Ok(json!({
            "status": "success",
            "operation": "workspace_risk_report",
            "data": {
                "root": root.to_string_lossy().to_string(),
                "diagnostics": diagnostics["data"].clone(),
                "dependency_hotspots": hotspots["data"].clone(),
                "risk_files": impacts
            }
        }))
    }

    pub fn recent_change_report(
        &self,
        path: String,
        max_results: Option<usize>,
    ) -> Result<Value, Value> {
        let canonical = validate_single_path(&self.resolver, &path).map_err(|e| e.to_value())?;
        let root = Self::workspace_root(Path::new(&canonical));
        let max_results = max_results.unwrap_or(50).clamp(1, 500);
        let mut items = Vec::new();
        let git = crate::tools::vcs::GitOperations::default();
        let diff = git.git_diff(Some(root.to_string_lossy().to_string())).unwrap_or_else(|e| {
            json!({
                "status": "error",
                "message": e
            })
        });
        let log = git.git_log(Some(root.to_string_lossy().to_string()), Some(max_results)).unwrap_or_else(|e| {
            json!({
                "status": "error",
                "message": e
            })
        });

        for file in Self::collect_workspace_files(&root).into_iter().take(max_results) {
            let content = match read_text_file(&file) {
                Ok(content) => content,
                Err(_) => continue,
            };
            let (total_lines, code_lines, function_count, cyclomatic) = Self::count_lines_and_complexity(&content);
            let score = code_lines + function_count + cyclomatic;
            items.push(json!({
                "file": file.to_string_lossy().to_string(),
                "score": score,
                "total_lines": total_lines,
                "code_lines": code_lines,
                "function_count": function_count,
                "cyclomatic_complexity_estimate": cyclomatic
            }));
        }

        items.sort_by(|a, b| b["score"].as_u64().unwrap_or(0).cmp(&a["score"].as_u64().unwrap_or(0)));

        Ok(json!({
            "status": "success",
            "operation": "recent_change_report",
            "data": {
                "root": root.to_string_lossy().to_string(),
                "recent_commits": log["data"].clone(),
                "working_tree_diff": diff["data"].clone(),
                "risk_files": items.into_iter().take(max_results).collect::<Vec<_>>()
            }
        }))
    }
}

fn tool_error(message: String) -> Value {
    json!({
        "error": {
            "code": "tool_error",
            "message": message
        }
    })
}
