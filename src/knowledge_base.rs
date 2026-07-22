//! Durable, workspace-scoped knowledge base with semantic chunks and hybrid retrieval.
//!
//! The default backend is deliberately local: BM25 provides exact-term recall while a
//! deterministic feature-hashing vector provides semantic recall without sending documents to a
//! third party. The persisted schema keeps the vector field replaceable by a hosted embedding or
//! vector database later.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::project_index;

const SCHEMA_VERSION: u32 = 2;
const LEGACY_SCHEMA_VERSION: u32 = 1;
const VECTOR_DIMENSION: usize = 512;
const TARGET_CHARS: usize = 1_200;
const MAX_CHARS: usize = 1_800;
const OVERLAP_CHARS: usize = 160;
const MAX_DOCUMENT_BYTES: usize = 64 * 1024 * 1024;
const DEFAULT_STALE_DAYS: i64 = 90;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeManifest {
    pub schema_version: u32,
    pub updated_at: DateTime<Utc>,
    pub documents: BTreeMap<String, KnowledgeDocument>,
}

impl Default for KnowledgeManifest {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            updated_at: Utc::now(),
            documents: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeDocument {
    pub id: String,
    pub name: String,
    pub format: String,
    pub source_path: String,
    pub content_hash: String,
    pub version: u32,
    pub status: KnowledgeStatus,
    pub owner: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_verified_at: DateTime<Utc>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub previous_version: Option<String>,
    pub byte_size: usize,
    pub chunks: Vec<KnowledgeChunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeStatus {
    Active,
    Stale,
    Archived,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeChunk {
    pub id: String,
    pub document_id: String,
    pub ordinal: usize,
    pub location: String,
    pub heading_path: Vec<String>,
    pub text: String,
    pub token_count: usize,
    pub entities: Vec<String>,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct KnowledgeMetadataInput {
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeState {
    pub schema_version: u32,
    pub updated_at: DateTime<Utc>,
    pub active: usize,
    pub stale: usize,
    pub expired: usize,
    pub archived: usize,
    pub chunks: usize,
    pub supported_formats: Vec<&'static str>,
    pub documents: Vec<KnowledgeDocumentSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeDocumentSummary {
    pub id: String,
    pub name: String,
    pub format: String,
    pub version: u32,
    pub status: KnowledgeStatus,
    pub owner: String,
    pub tags: Vec<String>,
    pub byte_size: usize,
    pub chunk_count: usize,
    pub updated_at: DateTime<Utc>,
    pub last_verified_at: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeSearchHit {
    pub document_id: String,
    pub document_name: String,
    pub chunk_id: String,
    pub location: String,
    pub heading_path: Vec<String>,
    pub text: String,
    pub score: f64,
    pub lexical_score: f64,
    pub semantic_score: f64,
    pub freshness_score: f64,
    pub version: u32,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
struct Candidate<'a> {
    document: &'a KnowledgeDocument,
    chunk: &'a KnowledgeChunk,
    lexical: f64,
    semantic: f64,
}

pub fn supported_formats() -> Vec<&'static str> {
    vec![
        "pdf", "docx", "pptx", "xlsx", "csv", "tsv", "md", "txt", "rst", "html", "xml", "json",
        "yaml", "yml", "toml", "tex", "bib", "sql", "rs", "py", "js", "ts",
    ]
}

pub fn root(workspace: &Path) -> PathBuf {
    workspace.join(".atlas").join("knowledge-base")
}

fn manifest_path(workspace: &Path) -> PathBuf {
    root(workspace).join("manifest.json")
}

pub fn load(workspace: &Path) -> Result<KnowledgeManifest> {
    let path = manifest_path(workspace);
    if !path.exists() {
        return Ok(KnowledgeManifest::default());
    }
    let bytes = fs::read(&path).with_context(|| format!("read {}", path.display()))?;
    let mut manifest: KnowledgeManifest = serde_json::from_slice(&bytes)?;
    if !matches!(
        manifest.schema_version,
        LEGACY_SCHEMA_VERSION | SCHEMA_VERSION
    ) {
        return Err(anyhow!(
            "unsupported knowledge base schema version {}",
            manifest.schema_version
        ));
    }
    if manifest.schema_version == LEGACY_SCHEMA_VERSION {
        // Schema v2 increased the local feature-hashing vector from 192 to 512
        // dimensions. Rebuild legacy vectors from their persisted source text so
        // existing workspaces remain readable without deleting or re-importing data.
        for chunk in manifest
            .documents
            .values_mut()
            .flat_map(|document| document.chunks.iter_mut())
        {
            chunk.vector = semantic_vector(&chunk.text);
        }
        manifest.schema_version = SCHEMA_VERSION;
    }
    refresh_statuses(&mut manifest);
    Ok(manifest)
}

pub fn state(workspace: &Path) -> Result<KnowledgeState> {
    let manifest = load(workspace)?;
    let mut documents = manifest.documents.values().map(summary).collect::<Vec<_>>();
    documents.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    let count = |status: KnowledgeStatus| {
        manifest
            .documents
            .values()
            .filter(|document| document.status == status)
            .count()
    };
    Ok(KnowledgeState {
        schema_version: manifest.schema_version,
        updated_at: manifest.updated_at,
        active: count(KnowledgeStatus::Active),
        stale: count(KnowledgeStatus::Stale),
        expired: count(KnowledgeStatus::Expired),
        archived: count(KnowledgeStatus::Archived),
        chunks: manifest
            .documents
            .values()
            .map(|item| item.chunks.len())
            .sum(),
        supported_formats: supported_formats(),
        documents,
    })
}

pub fn ingest_bytes(
    workspace: &Path,
    filename: &str,
    bytes: &[u8],
    metadata: KnowledgeMetadataInput,
) -> Result<KnowledgeDocumentSummary> {
    if bytes.is_empty() {
        return Err(anyhow!("document is empty"));
    }
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(anyhow!("document exceeds the 64 MiB knowledge-base limit"));
    }
    let safe_name = safe_filename(filename)?;
    let extension = Path::new(&safe_name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !supported_formats().contains(&extension.as_str()) {
        return Err(anyhow!("unsupported document format: .{}", extension));
    }
    let now = Utc::now();
    if metadata.valid_until.is_some_and(|until| until <= now) {
        return Err(anyhow!("valid_until must be in the future when uploading"));
    }
    let content_hash = blake3::hash(bytes).to_hex().to_string();
    let logical_key = safe_name.to_ascii_lowercase();
    let mut manifest = load(workspace)?;
    if let Some(existing) = manifest
        .documents
        .values()
        .find(|item| {
            item.name.to_ascii_lowercase() == logical_key && item.content_hash == content_hash
        })
        .cloned()
    {
        return Ok(summary(&existing));
    }
    let previous = manifest
        .documents
        .values()
        .filter(|item| item.name.to_ascii_lowercase() == logical_key)
        .max_by_key(|item| item.version)
        .cloned();
    let version = previous.as_ref().map(|item| item.version + 1).unwrap_or(1);
    let identity = format!("{}:{}:{}", logical_key, version, content_hash);
    let id = format!("kb_{}", &blake3::hash(identity.as_bytes()).to_hex()[..24]);
    let source_dir = root(workspace).join("sources");
    fs::create_dir_all(&source_dir)?;
    let stored_name = format!("{}-v{}.{}", id, version, extension);
    let stored_path = source_dir.join(&stored_name);
    let temporary = source_dir.join(format!("{}.tmp", stored_name));
    fs::write(&temporary, bytes)?;
    replace_file(&temporary, &stored_path)?;

    let parsed = project_index::parse_document(&stored_path)
        .with_context(|| format!("parse uploaded document {}", safe_name))?;
    let mut chunks = Vec::new();
    let mut headings = Vec::new();
    for parsed_chunk in parsed {
        for (location, heading_path, text) in
            semantic_chunks(&parsed_chunk.location, &parsed_chunk.text, &mut headings)
        {
            let ordinal = chunks.len();
            let chunk_identity = format!("{}:{}:{}", id, ordinal, text);
            chunks.push(KnowledgeChunk {
                id: format!(
                    "kc_{}",
                    &blake3::hash(chunk_identity.as_bytes()).to_hex()[..24]
                ),
                document_id: id.clone(),
                ordinal,
                location,
                heading_path,
                token_count: tokenize(&text).len(),
                entities: extract_entities(&text),
                vector: semantic_vector(&text),
                text,
            });
        }
    }
    if chunks.is_empty() {
        let _ = fs::remove_file(&stored_path);
        return Err(anyhow!(
            "no searchable text could be extracted from the document"
        ));
    }
    if let Some(previous) = previous.as_ref() {
        if let Some(item) = manifest.documents.get_mut(&previous.id) {
            item.status = KnowledgeStatus::Archived;
        }
    }
    let source_path = stored_path
        .strip_prefix(workspace)
        .unwrap_or(&stored_path)
        .to_string_lossy()
        .replace('\\', "/");
    let document = KnowledgeDocument {
        id: id.clone(),
        name: safe_name,
        format: extension,
        source_path,
        content_hash,
        version,
        status: KnowledgeStatus::Active,
        owner: non_empty(&metadata.owner, "workspace"),
        tags: normalize_tags(metadata.tags),
        created_at: previous.as_ref().map(|item| item.created_at).unwrap_or(now),
        updated_at: now,
        last_verified_at: now,
        valid_from: metadata.valid_from,
        valid_until: metadata.valid_until,
        previous_version: previous.map(|item| item.id),
        byte_size: bytes.len(),
        chunks,
    };
    let output = summary(&document);
    manifest.documents.insert(id, document);
    save(workspace, &mut manifest)?;
    Ok(output)
}

pub fn govern(
    workspace: &Path,
    document_id: &str,
    action: &str,
    metadata: Option<KnowledgeMetadataInput>,
) -> Result<KnowledgeDocumentSummary> {
    let mut manifest = load(workspace)?;
    let document = manifest
        .documents
        .get_mut(document_id)
        .ok_or_else(|| anyhow!("knowledge document not found: {}", document_id))?;
    match action.trim().to_ascii_lowercase().as_str() {
        "archive" => document.status = KnowledgeStatus::Archived,
        "restore" => {
            document.status = if document
                .valid_until
                .is_some_and(|until| until <= Utc::now())
            {
                KnowledgeStatus::Expired
            } else {
                KnowledgeStatus::Active
            };
        }
        "verify" => {
            document.last_verified_at = Utc::now();
            if document.status == KnowledgeStatus::Stale {
                document.status = KnowledgeStatus::Active;
            }
        }
        "metadata" => {
            let metadata = metadata.ok_or_else(|| anyhow!("metadata action requires metadata"))?;
            if !metadata.owner.trim().is_empty() {
                document.owner = metadata.owner.trim().to_string();
            }
            document.tags = normalize_tags(metadata.tags);
            document.valid_from = metadata.valid_from;
            document.valid_until = metadata.valid_until;
        }
        other => return Err(anyhow!("unknown knowledge governance action: {}", other)),
    }
    document.updated_at = Utc::now();
    let output = summary(document);
    save(workspace, &mut manifest)?;
    Ok(output)
}

pub fn search(workspace: &Path, query: &str, limit: usize) -> Result<Vec<KnowledgeSearchHit>> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let manifest = load(workspace)?;
    let active = manifest
        .documents
        .values()
        .filter(|item| {
            matches!(
                item.status,
                KnowledgeStatus::Active | KnowledgeStatus::Stale
            )
        })
        .collect::<Vec<_>>();
    let total_chunks = active
        .iter()
        .map(|item| item.chunks.len())
        .sum::<usize>()
        .max(1);
    let avg_len = active
        .iter()
        .flat_map(|item| &item.chunks)
        .map(|chunk| chunk.token_count)
        .sum::<usize>() as f64
        / total_chunks as f64;
    let query_terms = tokenize(query);
    let query_vector = semantic_vector(query);
    let mut document_frequency = HashMap::<String, usize>::new();
    for chunk in active.iter().flat_map(|item| &item.chunks) {
        let terms = tokenize(&chunk.text).into_iter().collect::<BTreeSet<_>>();
        for term in terms {
            *document_frequency.entry(term).or_default() += 1;
        }
    }
    let mut candidates = Vec::new();
    for document in active {
        for chunk in &document.chunks {
            let lexical = bm25(
                &query_terms,
                &tokenize(&format!(
                    "{} {} {}",
                    document.name,
                    chunk.heading_path.join(" "),
                    chunk.text
                )),
                &document_frequency,
                total_chunks,
                avg_len,
            );
            let semantic = cosine(&query_vector, &chunk.vector).max(0.0);
            if lexical > 0.0 || semantic >= 0.08 {
                candidates.push(Candidate {
                    document,
                    chunk,
                    lexical,
                    semantic,
                });
            }
        }
    }
    let mut lexical_order = (0..candidates.len()).collect::<Vec<_>>();
    lexical_order
        .sort_by(|left, right| float_cmp(candidates[*right].lexical, candidates[*left].lexical));
    let mut semantic_order = (0..candidates.len()).collect::<Vec<_>>();
    semantic_order
        .sort_by(|left, right| float_cmp(candidates[*right].semantic, candidates[*left].semantic));
    let lexical_rank = ranks(&lexical_order);
    let semantic_rank = ranks(&semantic_order);
    let now = Utc::now();
    let mut hits = candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            let freshness = freshness_score(candidate.document, now);
            let fused = 0.48 / (60.0 + lexical_rank[&index] as f64)
                + 0.42 / (60.0 + semantic_rank[&index] as f64)
                + 0.10 * freshness / 60.0;
            KnowledgeSearchHit {
                document_id: candidate.document.id.clone(),
                document_name: candidate.document.name.clone(),
                chunk_id: candidate.chunk.id.clone(),
                location: candidate.chunk.location.clone(),
                heading_path: candidate.chunk.heading_path.clone(),
                text: candidate.chunk.text.clone(),
                score: fused,
                lexical_score: candidate.lexical,
                semantic_score: candidate.semantic,
                freshness_score: freshness,
                version: candidate.document.version,
                updated_at: candidate.document.updated_at,
                tags: candidate.document.tags.clone(),
            }
        })
        .collect::<Vec<_>>();
    hits.sort_by(|left, right| float_cmp(right.score, left.score));
    hits.truncate(limit.clamp(1, 30));
    Ok(hits)
}

/// Rank short memory strings with the same lexical/semantic signals used by RAG.
pub fn rank_memory_texts(query: &str, memories: &[String], limit: usize) -> Vec<String> {
    let query_terms = tokenize(query);
    let query_vector = semantic_vector(query);
    let mut scored = memories
        .iter()
        .filter(|item| !item.trim().is_empty())
        .map(|item| {
            let terms = tokenize(item);
            let overlap = query_terms
                .iter()
                .filter(|term| terms.contains(term))
                .count() as f64
                / query_terms.len().max(1) as f64;
            let semantic = cosine(&query_vector, &semantic_vector(item)).max(0.0);
            (item.clone(), overlap * 0.55 + semantic * 0.45)
        })
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| float_cmp(right.1, left.1));
    scored
        .into_iter()
        .filter(|(_, score)| query_terms.is_empty() || *score >= 0.04)
        .take(limit)
        .map(|item| item.0)
        .collect()
}

pub fn prompt_context(workspace: &Path, query: &str, limit: usize) -> Result<String> {
    let hits = search(workspace, query, limit)?;
    if hits.is_empty() {
        return Ok(String::new());
    }
    let mut context = String::from(
        "Knowledge-base evidence (hybrid lexical + semantic retrieval; cite document and location):\n",
    );
    for hit in hits {
        context.push_str(&format!(
            "- [{} v{} | {} | freshness={:.2}] {}\n  {}\n",
            hit.document_name,
            hit.version,
            hit.location,
            hit.freshness_score,
            hit.heading_path.join(" > "),
            truncate(&hit.text, 1_100)
        ));
    }
    context.push_str("Treat expired or archived knowledge as unavailable. Prefer newer verified versions when evidence conflicts.");
    Ok(context)
}

fn save(workspace: &Path, manifest: &mut KnowledgeManifest) -> Result<()> {
    manifest.updated_at = Utc::now();
    refresh_statuses(manifest);
    let directory = root(workspace);
    fs::create_dir_all(&directory)?;
    let path = manifest_path(workspace);
    let temporary = directory.join("manifest.json.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(manifest)?)?;
    replace_file(&temporary, &path)
}

fn refresh_statuses(manifest: &mut KnowledgeManifest) {
    let now = Utc::now();
    for document in manifest.documents.values_mut() {
        if document.status == KnowledgeStatus::Archived {
            continue;
        }
        document.status = if document.valid_until.is_some_and(|until| until <= now) {
            KnowledgeStatus::Expired
        } else if now - document.last_verified_at > Duration::days(DEFAULT_STALE_DAYS) {
            KnowledgeStatus::Stale
        } else {
            KnowledgeStatus::Active
        };
    }
}

fn summary(document: &KnowledgeDocument) -> KnowledgeDocumentSummary {
    KnowledgeDocumentSummary {
        id: document.id.clone(),
        name: document.name.clone(),
        format: document.format.clone(),
        version: document.version,
        status: document.status.clone(),
        owner: document.owner.clone(),
        tags: document.tags.clone(),
        byte_size: document.byte_size,
        chunk_count: document.chunks.len(),
        updated_at: document.updated_at,
        last_verified_at: document.last_verified_at,
        valid_until: document.valid_until,
    }
}

fn semantic_chunks(
    base_location: &str,
    text: &str,
    headings: &mut Vec<String>,
) -> Vec<(String, Vec<String>, String)> {
    let mut units = Vec::<(Vec<String>, String)>::new();
    let mut paragraph = String::new();
    let flush =
        |paragraph: &mut String, headings: &[String], units: &mut Vec<(Vec<String>, String)>| {
            let value = paragraph.trim();
            if !value.is_empty() {
                units.push((headings.to_vec(), value.to_string()));
            }
            paragraph.clear();
        };
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some((level, title)) = markdown_heading(trimmed) {
            flush(&mut paragraph, headings, &mut units);
            headings.truncate(level.saturating_sub(1));
            headings.push(title.to_string());
            continue;
        }
        if trimmed.is_empty() {
            flush(&mut paragraph, headings, &mut units);
        } else {
            if !paragraph.is_empty() {
                paragraph.push('\n');
            }
            paragraph.push_str(trimmed);
        }
    }
    flush(&mut paragraph, headings, &mut units);
    if units.is_empty() && !text.trim().is_empty() {
        units.push((headings.to_vec(), text.trim().to_string()));
    }

    let mut output = Vec::new();
    let mut current = String::new();
    let mut current_headings = Vec::new();
    let push_current = |output: &mut Vec<(String, Vec<String>, String)>,
                        current: &mut String,
                        current_headings: &[String]| {
        if current.trim().is_empty() {
            return;
        }
        let index = output.len() + 1;
        output.push((
            format!("{} · semantic chunk {}", base_location, index),
            current_headings.to_vec(),
            current.trim().to_string(),
        ));
        *current = overlap_tail(current, OVERLAP_CHARS);
    };
    for (unit_headings, unit) in units {
        let pieces = split_long_unit(&unit);
        for piece in pieces {
            let heading_changed = !current.is_empty() && current_headings != unit_headings;
            if (heading_changed && current.chars().count() >= TARGET_CHARS / 2)
                || current.chars().count() + piece.chars().count() > MAX_CHARS
            {
                push_current(&mut output, &mut current, &current_headings);
            }
            if current.is_empty() || heading_changed {
                current_headings = unit_headings.clone();
            }
            if !current.is_empty() {
                current.push_str("\n\n");
            }
            current.push_str(&piece);
            if current.chars().count() >= TARGET_CHARS {
                push_current(&mut output, &mut current, &current_headings);
            }
        }
    }
    if !current.trim().is_empty() {
        let index = output.len() + 1;
        output.push((
            format!("{} · semantic chunk {}", base_location, index),
            current_headings,
            current.trim().to_string(),
        ));
    }
    output
}

fn split_long_unit(text: &str) -> Vec<String> {
    // Reserve two characters for the paragraph separator that is inserted
    // after an overlap tail, so an emitted chunk never exceeds the declared
    // maximum plus its overlap budget.
    let piece_limit = MAX_CHARS.saturating_sub(2);
    if text.chars().count() <= piece_limit {
        return vec![text.to_string()];
    }
    let mut output = Vec::new();
    let mut current = String::new();
    for sentence in text.split_inclusive(['.', '!', '?', '。', '！', '？', ';', '；']) {
        if current.chars().count() + sentence.chars().count() > piece_limit && !current.is_empty() {
            output.push(std::mem::take(&mut current));
        }
        if sentence.chars().count() > piece_limit {
            let chars = sentence.chars().collect::<Vec<_>>();
            for slice in chars.chunks(piece_limit) {
                output.push(slice.iter().collect());
            }
        } else {
            current.push_str(sentence);
        }
    }
    if !current.trim().is_empty() {
        output.push(current);
    }
    output
}

fn markdown_heading(line: &str) -> Option<(usize, &str)> {
    let level = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if (1..=6).contains(&level) {
        let title = line[level..].trim();
        if !title.is_empty() {
            return Some((level, title));
        }
    }
    None
}

fn overlap_tail(text: &str, count: usize) -> String {
    let chars = text.chars().collect::<Vec<_>>();
    chars[chars.len().saturating_sub(count)..].iter().collect()
}

fn tokenize(text: &str) -> Vec<String> {
    let lowered = text.to_lowercase();
    let mut tokens = lowered
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .filter(|token| token.chars().count() >= 2)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let cjk = lowered
        .chars()
        .filter(|character| ('\u{3400}'..='\u{9fff}').contains(character))
        .collect::<Vec<_>>();
    tokens.extend(cjk.windows(2).map(|window| window.iter().collect()));
    tokens
}

fn semantic_vector(text: &str) -> Vec<f32> {
    let mut vector = vec![0.0f32; VECTOR_DIMENSION];
    for token in tokenize(text) {
        let digest = blake3::hash(token.as_bytes());
        let bytes = digest.as_bytes();
        let index = u16::from_le_bytes([bytes[0], bytes[1]]) as usize % VECTOR_DIMENSION;
        let sign = if bytes[2] & 1 == 0 { 1.0 } else { -1.0 };
        vector[index] += sign * (1.0 + (token.chars().count().min(12) as f32 / 12.0));
    }
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut vector {
            *value /= norm;
        }
    }
    vector
}

fn cosine(left: &[f32], right: &[f32]) -> f64 {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| *left as f64 * *right as f64)
        .sum()
}

fn bm25(
    query: &[String],
    document: &[String],
    document_frequency: &HashMap<String, usize>,
    total_documents: usize,
    average_length: f64,
) -> f64 {
    let mut frequencies = HashMap::<&str, usize>::new();
    for term in document {
        *frequencies.entry(term).or_default() += 1;
    }
    let length = document.len().max(1) as f64;
    query
        .iter()
        .map(|term| {
            let frequency = *frequencies.get(term.as_str()).unwrap_or(&0) as f64;
            if frequency == 0.0 {
                return 0.0;
            }
            let df = *document_frequency.get(term).unwrap_or(&0) as f64;
            let idf = ((total_documents as f64 - df + 0.5) / (df + 0.5) + 1.0).ln();
            let k1 = 1.5;
            let b = 0.75;
            idf * frequency * (k1 + 1.0)
                / (frequency + k1 * (1.0 - b + b * length / average_length.max(1.0)))
        })
        .sum()
}

fn freshness_score(document: &KnowledgeDocument, now: DateTime<Utc>) -> f64 {
    if document.status == KnowledgeStatus::Expired || document.status == KnowledgeStatus::Archived {
        return 0.0;
    }
    let age = (now - document.last_verified_at).num_days().max(0) as f64;
    let decay = (-age / 120.0).exp();
    if document.status == KnowledgeStatus::Stale {
        decay * 0.55
    } else {
        decay
    }
}

fn ranks(order: &[usize]) -> HashMap<usize, usize> {
    order
        .iter()
        .enumerate()
        .map(|(rank, index)| (*index, rank + 1))
        .collect()
}

fn extract_entities(text: &str) -> Vec<String> {
    let mut entities = BTreeSet::new();
    for token in text.split_whitespace() {
        let cleaned = token.trim_matches(|character: char| {
            !character.is_alphanumeric() && character != '_' && character != '-'
        });
        let is_identifier = cleaned.contains('_') || cleaned.contains('-');
        let capitalized = cleaned.chars().next().is_some_and(char::is_uppercase);
        if cleaned.chars().count() >= 3 && (is_identifier || capitalized) {
            entities.insert(cleaned.to_string());
        }
        if entities.len() >= 24 {
            break;
        }
    }
    entities.into_iter().collect()
}

fn safe_filename(filename: &str) -> Result<String> {
    let trimmed = filename.trim();
    let path = Path::new(trimmed);
    if trimmed.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(anyhow!(
            "document filename must be a single safe path component"
        ));
    }
    let value = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("document filename is invalid"))?;
    Ok(value
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || matches!(character, '.' | '-' | '_' | ' ') {
                character
            } else {
                '_'
            }
        })
        .collect())
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    tags.into_iter()
        .map(|tag| tag.trim().to_ascii_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .take(32)
        .collect()
}

fn non_empty(value: &str, fallback: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn float_cmp(left: f64, right: f64) -> Ordering {
    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}

fn truncate(text: &str, max: usize) -> String {
    let mut output = text.chars().take(max).collect::<String>();
    if text.chars().count() > max {
        output.push('…');
    }
    output
}

fn replace_file(temporary: &Path, target: &Path) -> Result<()> {
    if target.exists() {
        fs::remove_file(target)?;
    }
    fs::rename(temporary, target)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_chunking_respects_headings_and_size() {
        let mut headings = Vec::new();
        let text = format!(
            "# Retrieval\n\n{}\n\n## Governance\n\n{}",
            "semantic evidence. ".repeat(100),
            "retention policy. ".repeat(100)
        );
        let chunks = semantic_chunks("document", &text, &mut headings);
        assert!(chunks.len() >= 2);
        assert!(chunks
            .iter()
            .all(|item| item.2.chars().count() <= MAX_CHARS + OVERLAP_CHARS));
        assert!(chunks
            .iter()
            .any(|item| item.1.iter().any(|heading| heading == "Governance")));
    }

    #[test]
    fn knowledge_lifecycle_and_hybrid_search_are_persistent() {
        let directory = tempfile::tempdir().unwrap();
        let document = ingest_bytes(
            directory.path(),
            "architecture.md",
            b"# Search\n\nHybrid retrieval combines semantic vectors and BM25 lexical recall.\n\n# Governance\n\nArchived knowledge is excluded.",
            KnowledgeMetadataInput {
                owner: "platform".into(),
                tags: vec!["RAG".into()],
                ..Default::default()
            },
        )
        .unwrap();
        let hits = search(directory.path(), "semantic search", 5).unwrap();
        assert!(!hits.is_empty());
        assert_eq!(hits[0].document_name, "architecture.md");
        govern(directory.path(), &document.id, "archive", None).unwrap();
        assert!(search(directory.path(), "semantic search", 5)
            .unwrap()
            .is_empty());
        govern(directory.path(), &document.id, "restore", None).unwrap();
        assert!(!search(directory.path(), "BM25 recall", 5)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn context_aware_memory_ranking_prefers_related_items() {
        let memories = vec![
            "User prefers Chinese answers".to_string(),
            "Refund requests use the US policy".to_string(),
            "Python tests run with pytest".to_string(),
        ];
        let ranked = rank_memory_texts("fix the Python test", &memories, 2);
        assert_eq!(ranked[0], "Python tests run with pytest");
    }

    #[test]
    fn schema_v1_manifest_is_migrated_in_memory_and_future_versions_are_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let manifest_directory = root(directory.path());
        fs::create_dir_all(&manifest_directory).unwrap();

        let mut legacy = KnowledgeManifest::default();
        legacy.schema_version = LEGACY_SCHEMA_VERSION;
        fs::write(
            manifest_directory.join("manifest.json"),
            serde_json::to_vec_pretty(&legacy).unwrap(),
        )
        .unwrap();
        assert_eq!(
            load(directory.path()).unwrap().schema_version,
            SCHEMA_VERSION
        );
        assert_eq!(semantic_vector("legacy knowledge").len(), VECTOR_DIMENSION);

        legacy.schema_version = SCHEMA_VERSION + 1;
        fs::write(
            manifest_directory.join("manifest.json"),
            serde_json::to_vec_pretty(&legacy).unwrap(),
        )
        .unwrap();
        assert!(load(directory.path())
            .unwrap_err()
            .to_string()
            .contains("unsupported knowledge base schema version"));
    }
}
