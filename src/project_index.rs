use anyhow::{anyhow, Context, Result};
use lopdf::Document;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use walkdir::{DirEntry, WalkDir};
use zip::ZipArchive;

const INDEX_VERSION: u32 = 2;
const CHUNK_CHARS: usize = 4_000;
const MAX_INDEX_FILE_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectIndex {
    pub version: u32,
    pub updated_at: i64,
    pub files: BTreeMap<String, IndexedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedFile {
    pub path: String,
    pub kind: String,
    pub size: u64,
    pub modified_ns: u128,
    pub chunks: Vec<IndexChunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexChunk {
    pub location: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexUpdate {
    pub scanned: usize,
    pub indexed: usize,
    pub unchanged: usize,
    pub removed: usize,
    pub skipped: usize,
    pub files: usize,
    pub chunks: usize,
}

struct PendingFile {
    path: PathBuf,
    relative: String,
    kind: &'static str,
    size: u64,
    modified_ns: u128,
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexSearchHit {
    pub path: String,
    pub kind: String,
    pub location: String,
    pub score: usize,
    pub snippet: String,
}

pub fn index_dir(workspace: &Path) -> PathBuf {
    workspace.join(".tokitai").join("index")
}
pub fn index_path(workspace: &Path) -> PathBuf {
    index_dir(workspace).join("manifest.json")
}

pub fn load(workspace: &Path) -> Result<ProjectIndex> {
    let path = index_path(workspace);
    if !path.exists() {
        return Ok(ProjectIndex {
            version: INDEX_VERSION,
            ..Default::default()
        });
    }
    let bytes = fs::read(&path).with_context(|| format!("read {}", path.display()))?;
    let parsed: ProjectIndex = serde_json::from_slice(&bytes)?;
    if parsed.version != INDEX_VERSION {
        return Ok(ProjectIndex {
            version: INDEX_VERSION,
            ..Default::default()
        });
    }
    Ok(parsed)
}

pub fn update(workspace: &Path) -> Result<IndexUpdate> {
    let workspace = workspace.canonicalize()?;
    let mut prior = load(&workspace)?;
    let mut next = BTreeMap::new();
    let mut pending = Vec::new();
    let (mut scanned, mut indexed, mut unchanged, mut skipped) = (0, 0, 0, 0);
    for entry in WalkDir::new(&workspace)
        .follow_links(false)
        .into_iter()
        .filter_entry(eligible_entry)
    {
        let entry = match entry {
            Ok(v) => v,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(&workspace)?
            .to_string_lossy()
            .replace('\\', "/");
        let Some(kind) = supported_kind(entry.path()) else {
            continue;
        };
        scanned += 1;
        let meta = entry.metadata()?;
        let modified_ns = meta
            .modified()
            .ok()
            .and_then(|v| v.duration_since(UNIX_EPOCH).ok())
            .map(|v| v.as_nanos())
            .unwrap_or(0);
        if let Some(old) = prior.files.remove(&relative) {
            if old.size == meta.len() && old.modified_ns == modified_ns {
                next.insert(relative, old);
                unchanged += 1;
                continue;
            }
        }
        if meta.len() > MAX_INDEX_FILE_BYTES
            && kind != "pdf"
            && kind != "docx"
            && kind != "spreadsheet"
        {
            skipped += 1;
            continue;
        }
        pending.push(PendingFile {
            path: entry.path().to_path_buf(),
            relative,
            kind,
            size: meta.len(),
            modified_ns,
        });
    }
    // Parsing PDFs, Office documents and source files is CPU intensive. Keep the directory walk
    // deterministic, then fan only the independent parsing work out across Rayon's bounded pool.
    let parsed: Vec<_> = pending
        .into_par_iter()
        .map(|file| {
            let chunks = parse_file(&file.path, file.kind);
            (file, chunks)
        })
        .collect();
    for (file, chunks) in parsed {
        match chunks {
            Ok(chunks) if !chunks.is_empty() => {
                next.insert(
                    file.relative.clone(),
                    IndexedFile {
                        path: file.relative,
                        kind: file.kind.into(),
                        size: file.size,
                        modified_ns: file.modified_ns,
                        chunks,
                    },
                );
                indexed += 1;
            }
            _ => skipped += 1,
        }
    }
    let removed = prior.files.len();
    let now = chrono::Utc::now().timestamp();
    let index = ProjectIndex {
        version: INDEX_VERSION,
        updated_at: now,
        files: next,
    };
    fs::create_dir_all(index_dir(&workspace))?;
    let temp = index_dir(&workspace).join("manifest.json.tmp");
    fs::write(&temp, serde_json::to_vec(&index)?)?;
    replace_file(&temp, &index_path(&workspace))?;
    Ok(IndexUpdate {
        scanned,
        indexed,
        unchanged,
        removed,
        skipped,
        files: index.files.len(),
        chunks: index.files.values().map(|v| v.chunks.len()).sum(),
    })
}

pub fn search(
    workspace: &Path,
    query: &str,
    limit: usize,
    kind: Option<&str>,
) -> Result<Vec<IndexSearchHit>> {
    let terms: Vec<String> = query
        .split_whitespace()
        .map(|v| v.to_lowercase())
        .filter(|v| !v.is_empty())
        .collect();
    if terms.is_empty() {
        return Ok(Vec::new());
    }
    let index = load(workspace)?;
    let mut hits = Vec::new();
    for file in index.files.values() {
        if kind.is_some_and(|filter| filter != file.kind) {
            continue;
        }
        let path_lower = file.path.to_lowercase();
        let path_score = terms
            .iter()
            .filter(|term| path_lower.contains(term.as_str()))
            .count()
            * 4;
        for chunk in &file.chunks {
            let lower = chunk.text.to_lowercase();
            let content_score: usize = terms
                .iter()
                .map(|term| lower.match_indices(term).count())
                .sum();
            let score = content_score + path_score;
            if score == 0 {
                continue;
            }
            let first = terms
                .iter()
                .filter_map(|t| lower.find(t))
                .min()
                .unwrap_or(0);
            let start = floor_char_boundary(&chunk.text, first.saturating_sub(180));
            let end = ceil_char_boundary(&chunk.text, (first + 420).min(chunk.text.len()));
            hits.push(IndexSearchHit {
                path: file.path.clone(),
                kind: file.kind.clone(),
                location: chunk.location.clone(),
                score,
                snippet: chunk.text[start..end].replace('\n', " "),
            });
        }
    }
    hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
    hits.truncate(limit.clamp(1, 100));
    Ok(hits)
}

fn eligible_entry(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return true;
    }
    !matches!(
        entry.file_name().to_string_lossy().as_ref(),
        ".git"
            | ".tokitai"
            | ".idea"
            | ".vscode"
            | ".cache"
            | ".next"
            | ".nuxt"
            | "target"
            | "node_modules"
            | ".venv"
            | "venv"
            | "dist"
            | "build"
            | "coverage"
            | "vendor"
    )
}

fn supported_kind(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    match ext.as_str() {
        "pdf" => Some("pdf"),
        "docx" => Some("docx"),
        "xlsx" => Some("spreadsheet"),
        "csv" | "tsv" => Some("spreadsheet"),
        "rs" | "py" | "js" | "mjs" | "cjs" | "ts" | "tsx" | "jsx" | "java" | "kt" | "go" | "c"
        | "h" | "cpp" | "hpp" | "cs" | "rb" | "php" | "swift" | "scala" | "sh" | "ps1" | "sql"
        | "html" | "css" | "scss" | "vue" | "svelte" => Some("code"),
        "md" | "txt" | "rst" | "toml" | "yaml" | "yml" | "json" | "xml" | "tex" | "bib" => {
            Some("document")
        }
        _ => None,
    }
}

fn parse_file(path: &Path, kind: &str) -> Result<Vec<IndexChunk>> {
    match path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "pdf" => parse_pdf(path),
        "docx" => parse_docx(path),
        "xlsx" => parse_xlsx(path),
        "csv" => parse_delimited(path, ','),
        "tsv" => parse_delimited(path, '\t'),
        _ => parse_text(path, kind),
    }
}

fn parse_pdf(path: &Path) -> Result<Vec<IndexChunk>> {
    let doc = Document::load(path)?;
    let mut out = Vec::new();
    for (page, _) in doc.get_pages() {
        if let Ok(text) = doc.extract_text(&[page]) {
            push_chunks(&mut out, format!("page {}", page), &text);
        }
    }
    Ok(out)
}

fn parse_docx(path: &Path) -> Result<Vec<IndexChunk>> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")?
        .read_to_string(&mut xml)?;
    let text = xml_text(&xml, &["</w:p>", "</w:tr>", "</w:tc>"]);
    let mut out = Vec::new();
    push_chunks(&mut out, "document".into(), &text);
    Ok(out)
}

fn parse_xlsx(path: &Path) -> Result<Vec<IndexChunk>> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let mut shared = Vec::new();
    if let Ok(mut file) = archive.by_name("xl/sharedStrings.xml") {
        let mut xml = String::new();
        file.read_to_string(&mut xml)?;
        shared = xml.split("</si>").map(|v| xml_text(v, &[])).collect();
    }
    let mut names = Vec::new();
    for i in 0..archive.len() {
        let name = archive.by_index(i)?.name().to_string();
        if name.starts_with("xl/worksheets/sheet") && name.ends_with(".xml") {
            names.push(name);
        }
    }
    let mut out = Vec::new();
    for name in names {
        let mut xml = String::new();
        archive.by_name(&name)?.read_to_string(&mut xml)?;
        let mut rows = String::new();
        for (row_idx, row) in xml.split("<row").skip(1).enumerate() {
            let cells: Vec<String> = row
                .split("<c ")
                .skip(1)
                .map(|cell| {
                    let value = between(cell, "<v>", "</v>").unwrap_or_default();
                    if cell
                        .split('>')
                        .next()
                        .is_some_and(|head| head.contains("t=\"s\""))
                    {
                        value
                            .parse::<usize>()
                            .ok()
                            .and_then(|i| shared.get(i).cloned())
                            .unwrap_or(value)
                    } else {
                        value
                    }
                })
                .collect();
            rows.push_str(&format!("row {}: {}\n", row_idx + 1, cells.join(" | ")));
        }
        push_chunks(
            &mut out,
            name.trim_start_matches("xl/worksheets/")
                .trim_end_matches(".xml")
                .into(),
            &rows,
        );
    }
    Ok(out)
}

fn parse_delimited(path: &Path, delimiter: char) -> Result<Vec<IndexChunk>> {
    let text = fs::read_to_string(path)?;
    let mut out = Vec::new();
    let mut block = String::new();
    let mut start = 1;
    for (i, line) in text.lines().enumerate() {
        if block.len() + line.len() > CHUNK_CHARS {
            out.push(IndexChunk {
                location: format!("rows {}-{}", start, i),
                text: std::mem::take(&mut block),
            });
            start = i + 1;
        }
        block.push_str(&line.replace(delimiter, " | "));
        block.push('\n');
    }
    if !block.is_empty() {
        out.push(IndexChunk {
            location: format!("rows {}-{}", start, text.lines().count()),
            text: block,
        });
    }
    Ok(out)
}

fn parse_text(path: &Path, _: &str) -> Result<Vec<IndexChunk>> {
    let bytes = fs::read(path)?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(anyhow!("file too large"));
    }
    let text = String::from_utf8_lossy(&bytes);
    let mut out = Vec::new();
    let mut block = String::new();
    let mut start = 1;
    for (i, line) in text.lines().enumerate() {
        if block.len() + line.len() > CHUNK_CHARS {
            out.push(IndexChunk {
                location: format!("lines {}-{}", start, i),
                text: std::mem::take(&mut block),
            });
            start = i + 1;
        }
        block.push_str(line);
        block.push('\n');
    }
    if !block.is_empty() {
        out.push(IndexChunk {
            location: format!("lines {}-{}", start, text.lines().count()),
            text: block,
        });
    }
    Ok(out)
}

fn push_chunks(out: &mut Vec<IndexChunk>, location: String, text: &str) {
    let mut start = 0;
    while start < text.len() {
        let end = floor_char_boundary(text, (start + CHUNK_CHARS).min(text.len()));
        if end <= start {
            break;
        }
        out.push(IndexChunk {
            location: location.clone(),
            text: text[start..end].to_string(),
        });
        start = end;
    }
}
fn between(value: &str, start: &str, end: &str) -> Option<String> {
    let at = value.find(start)? + start.len();
    let tail = &value[at..];
    Some(tail[..tail.find(end)?].to_string())
}
fn xml_text(xml: &str, breaks: &[&str]) -> String {
    let mut value = xml.to_string();
    for tag in breaks {
        value = value.replace(tag, "\n");
    }
    let mut out = String::new();
    let mut in_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
}
fn floor_char_boundary(value: &str, mut at: usize) -> usize {
    at = at.min(value.len());
    while at > 0 && !value.is_char_boundary(at) {
        at -= 1;
    }
    at
}
fn ceil_char_boundary(value: &str, mut at: usize) -> usize {
    at = at.min(value.len());
    while at < value.len() && !value.is_char_boundary(at) {
        at += 1;
    }
    at
}
fn replace_file(temp: &Path, target: &Path) -> Result<()> {
    if target.exists() {
        fs::remove_file(target)?;
    }
    fs::rename(temp, target)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    #[test]
    fn incrementally_reuses_and_removes_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("a.rs"),
            "fn atlas() { println!(\"needle\"); }",
        )
        .unwrap();
        let first = update(dir.path()).unwrap();
        assert_eq!(first.indexed, 1);
        let second = update(dir.path()).unwrap();
        assert_eq!(second.unchanged, 1);
        assert_eq!(
            search(dir.path(), "needle", 5, Some("code")).unwrap().len(),
            1
        );
        fs::remove_file(dir.path().join("a.rs")).unwrap();
        let third = update(dir.path()).unwrap();
        assert_eq!(third.removed, 1);
    }

    #[test]
    fn parses_docx_paragraphs_and_xlsx_rows() {
        let dir = tempfile::tempdir().unwrap();
        let docx = dir.path().join("paper.docx");
        let mut zip = zip::ZipWriter::new(File::create(&docx).unwrap());
        zip.start_file("word/document.xml", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(
            br#"<w:document><w:p><w:r><w:t>Atlas document needle</w:t></w:r></w:p></w:document>"#,
        )
        .unwrap();
        zip.finish().unwrap();
        let xlsx = dir.path().join("data.xlsx");
        let mut zip = zip::ZipWriter::new(File::create(&xlsx).unwrap());
        zip.start_file("xl/sharedStrings.xml", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(br#"<sst><si><t>metric needle</t></si></sst>"#)
            .unwrap();
        zip.start_file("xl/worksheets/sheet1.xml", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(br#"<worksheet><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1"><v>42</v></c></row></sheetData></worksheet>"#).unwrap();
        zip.finish().unwrap();
        let result = update(dir.path()).unwrap();
        assert_eq!(result.indexed, 2);
        let hits = search(dir.path(), "needle", 10, None).unwrap();
        assert!(hits.iter().any(|v| v.kind == "docx"));
        assert!(hits.iter().any(|v| v.kind == "spreadsheet"));
    }
}
