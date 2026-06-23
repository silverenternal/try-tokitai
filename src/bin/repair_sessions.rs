use std::fs;
use std::path::{Path, PathBuf};

use ai_assistant::app_paths::AppPaths;
use ai_assistant::tui::components::diff_viewer::DiffLine;
use ai_assistant::tui::components::message_block::MessageBlock;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionBranch {
    id: String,
    name: String,
    #[serde(default)]
    parent_id: String,
    fork_msg_index: usize,
    #[serde(default)]
    merged_into: Option<String>,
    color_idx: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionMeta {
    id: String,
    title: String,
    #[serde(default)]
    custom_title: bool,
    #[serde(default)]
    summary: String,
    created_at: String,
    updated_at: String,
    message_count: usize,
    model: String,
    #[serde(default)]
    branches: Vec<SessionBranch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionFile {
    meta: SessionMeta,
    messages: Vec<MessageBlock>,
}

#[derive(Debug, Default)]
struct RepairStats {
    scanned_files: usize,
    updated_files: usize,
    backed_up_files: usize,
    repaired_blocks: usize,
    repaired_nested_fields: usize,
    updated_titles: usize,
    updated_summaries: usize,
}

fn main() -> Result<()> {
    let options = parse_args()?;
    let sessions_dir = resolve_sessions_dir(options.state_dir.clone())?;
    fs::create_dir_all(&sessions_dir)?;

    let backup_root = sessions_dir.join("_repair_backup");
    let mut stats = RepairStats::default();

    let mut index = load_index(&sessions_dir)?;
    for meta in &mut index {
        let path = sessions_dir.join(format!("{}.json", meta.id));
        if !path.exists() {
            continue;
        }
        stats.scanned_files += 1;
        let changed = repair_session_file(&path, meta, options.dry_run, &backup_root, &mut stats)?;
        if changed {
            stats.updated_files += 1;
        }
    }

    if !options.dry_run {
        let index_path = sessions_dir.join("index.json");
        fs::write(&index_path, serde_json::to_string_pretty(&index)?)?;
    }

    println!("Session repair complete");
    println!("sessions_dir: {}", sessions_dir.display());
    println!("dry_run: {}", options.dry_run);
    println!("scanned_files: {}", stats.scanned_files);
    println!("updated_files: {}", stats.updated_files);
    println!("backed_up_files: {}", stats.backed_up_files);
    println!("repaired_blocks: {}", stats.repaired_blocks);
    println!("repaired_nested_fields: {}", stats.repaired_nested_fields);
    println!("updated_titles: {}", stats.updated_titles);
    println!("updated_summaries: {}", stats.updated_summaries);

    if !options.dry_run && stats.updated_files > 0 {
        println!("backup_dir: {}", backup_root.display());
    }

    Ok(())
}

#[derive(Debug, Default)]
struct RepairOptions {
    state_dir: Option<PathBuf>,
    dry_run: bool,
}

fn parse_args() -> Result<RepairOptions> {
    let mut options = RepairOptions::default();
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--dry-run" => options.dry_run = true,
            "--state-dir" => {
                let Some(value) = args.next() else {
                    anyhow::bail!("--state-dir requires a value");
                };
                options.state_dir = Some(PathBuf::from(value));
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => anyhow::bail!("unknown argument: {}", other),
        }
    }

    Ok(options)
}

fn print_help() {
    println!("Usage: cargo run --bin repair_sessions -- [--dry-run] [--state-dir <path>]");
    println!("  Once-off migration cleaner for legacy session corruption.");
    println!("  --dry-run            inspect and report only, do not modify files");
    println!("  --state-dir <path>   explicit Tokitai state directory containing sessions/");
}

fn resolve_sessions_dir(explicit_state_dir: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(state_dir) = explicit_state_dir {
        return Ok(state_dir.join("sessions"));
    }

    if let Ok(explicit) = std::env::var("TOKITAI_STATE_DIR") {
        let state_dir = PathBuf::from(explicit);
        return Ok(state_dir.join("sessions"));
    }

    let root = AppPaths::discover_project_root();
    let local_dev = AppPaths::for_local_dev(root.clone()).sessions_dir();
    if local_dev.exists() || root.join("Cargo.toml").exists() {
        return Ok(local_dev);
    }

    if let Some(paths) = AppPaths::for_desktop_defaults() {
        return Ok(paths.sessions_dir());
    }

    Ok(local_dev)
}

fn load_index(sessions_dir: &Path) -> Result<Vec<SessionMeta>> {
    let index_path = sessions_dir.join("index.json");
    if !index_path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&index_path)
        .with_context(|| format!("failed to read {}", index_path.display()))?;
    let index = serde_json::from_str::<Vec<SessionMeta>>(&raw)
        .with_context(|| format!("failed to parse {}", index_path.display()))?;
    Ok(index)
}

fn repair_session_file(
    path: &Path,
    meta: &mut SessionMeta,
    dry_run: bool,
    backup_root: &Path,
    stats: &mut RepairStats,
) -> Result<bool> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let mut file: SessionFile = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    let mut changed = false;

    for block in &mut file.messages {
        let repaired = repair_message_block(block, &mut stats.repaired_nested_fields);
        if repaired {
            stats.repaired_blocks += 1;
            changed = true;
        }
    }

    let next_title = auto_title(&file.messages).unwrap_or_else(|| "New conversation".to_string());
    if !file.meta.custom_title && file.meta.title != next_title {
        file.meta.title = next_title.clone();
        meta.title = next_title;
        stats.updated_titles += 1;
        changed = true;
    }

    let next_summary = auto_summary(&file.messages).unwrap_or_default();
    if file.meta.summary != next_summary || meta.summary != next_summary {
        file.meta.summary = next_summary.clone();
        meta.summary = next_summary;
        stats.updated_summaries += 1;
        changed = true;
    }

    file.meta.message_count = file.messages.len();
    meta.message_count = file.messages.len();
    meta.title = file.meta.title.clone();
    meta.summary = file.meta.summary.clone();

    if changed && !dry_run {
        backup_file(path, backup_root, &mut stats.backed_up_files)?;
        fs::write(path, serde_json::to_string_pretty(&file)?)?;
    }

    Ok(changed)
}

fn backup_file(path: &Path, backup_root: &Path, backed_up_files: &mut usize) -> Result<()> {
    fs::create_dir_all(backup_root)?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("session.json");
    let backup_path = backup_root.join(name);
    if !backup_path.exists() {
        fs::copy(path, &backup_path)
            .with_context(|| format!("failed to back up {} to {}", path.display(), backup_path.display()))?;
        *backed_up_files += 1;
    }
    Ok(())
}

fn repair_message_block(block: &mut MessageBlock, repaired_nested_fields: &mut usize) -> bool {
    match block {
        MessageBlock::User { content, .. }
        | MessageBlock::Assistant { content }
        | MessageBlock::AssistantStreaming { content }
        | MessageBlock::Thinking { content, .. }
        | MessageBlock::Error { content }
        | MessageBlock::System { content } => repair_text_in_place(content),
        MessageBlock::ToolResult { result, .. } => repair_text_in_place(result),
        MessageBlock::ToolCall { name, .. } => repair_text_in_place(name),
        MessageBlock::Subagent { record } => {
            let mut changed = false;
            changed |= repair_text_counted(&mut record.id, repaired_nested_fields);
            changed |= repair_text_counted(&mut record.name, repaired_nested_fields);
            changed |= repair_text_counted(&mut record.purpose, repaired_nested_fields);
            changed |= repair_text_counted(&mut record.input, repaired_nested_fields);
            changed |= repair_text_counted(&mut record.output, repaired_nested_fields);
            changed |= repair_text_counted(&mut record.status, repaired_nested_fields);
            changed |= repair_text_counted(&mut record.kind, repaired_nested_fields);
            changed |= repair_optional_text_counted(&mut record.started_at, repaired_nested_fields);
            changed |= repair_optional_text_counted(&mut record.completed_at, repaired_nested_fields);
            for evidence in &mut record.evidence {
                changed |= repair_text_counted(evidence, repaired_nested_fields);
            }
            changed
        }
        MessageBlock::Verification { report } => {
            let mut changed = false;
            changed |= repair_text_counted(&mut report.status, repaired_nested_fields);
            changed |= repair_text_counted(&mut report.summary, repaired_nested_fields);
            for issue in &mut report.issues {
                changed |= repair_text_counted(issue, repaired_nested_fields);
            }
            for evidence in &mut report.evidence {
                changed |= repair_text_counted(evidence, repaired_nested_fields);
            }
            for action in &mut report.next_actions {
                changed |= repair_text_counted(action, repaired_nested_fields);
            }
            for check in &mut report.checks {
                changed |= repair_text_counted(&mut check.id, repaired_nested_fields);
                changed |= repair_text_counted(&mut check.title, repaired_nested_fields);
                changed |= repair_text_counted(&mut check.status, repaired_nested_fields);
                changed |= repair_text_counted(&mut check.detail, repaired_nested_fields);
                for evidence in &mut check.evidence {
                    changed |= repair_text_counted(evidence, repaired_nested_fields);
                }
            }
            changed
        }
        MessageBlock::Diff { diff } => {
            let mut changed = false;
            changed |= repair_text_counted(&mut diff.file_path, repaired_nested_fields);
            changed |= repair_text_counted(&mut diff.before_content, repaired_nested_fields);
            changed |= repair_text_counted(&mut diff.after_content, repaired_nested_fields);
            for line in &mut diff.lines {
                match line {
                    DiffLine::Header(value)
                    | DiffLine::Context(value)
                    | DiffLine::Add(value)
                    | DiffLine::Remove(value) => {
                        changed |= repair_text_counted(value, repaired_nested_fields);
                    }
                }
            }
            changed
        }
    }
}

fn repair_text_counted(content: &mut String, repaired_nested_fields: &mut usize) -> bool {
    let changed = repair_text_in_place(content);
    if changed {
        *repaired_nested_fields += 1;
    }
    changed
}

fn repair_optional_text_counted(
    content: &mut Option<String>,
    repaired_nested_fields: &mut usize,
) -> bool {
    let Some(value) = content.as_mut() else {
        return false;
    };
    repair_text_counted(value, repaired_nested_fields)
}

fn repair_text_in_place(content: &mut String) -> bool {
    let original = content.clone();
    if !looks_like_corrupted_text(&original) {
        return false;
    }

    if let Some(restored) = try_restore_mojibake(&original) {
        *content = restored;
        return true;
    }

    *content = if is_question_mark_garbage(&original) {
        "[历史消息已因旧编码损坏而清洗，原文无法恢复]".to_string()
    } else {
        "[历史消息存在编码损坏，已清洗]".to_string()
    };
    true
}

fn compact_message_text(raw: &str, max_chars: usize) -> Option<String> {
    let cleaned = raw
        .replace("[AGENT]", "")
        .replace('\r', " ")
        .replace('\n', " ");
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");

    if cleaned.is_empty() {
        return None;
    }

    let clipped: String = cleaned.chars().take(max_chars).collect();
    Some(if cleaned.chars().count() > max_chars {
        format!("{}...", clipped.trim_end())
    } else {
        clipped.trim().to_string()
    })
}

fn looks_like_corrupted_text(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.contains('\u{fffd}') {
        return true;
    }
    let total_chars = trimmed.chars().count().max(1);
    let question_like = trimmed.chars().filter(|ch| matches!(ch, '?' | '？')).count();
    if question_like >= 4 && (question_like as f32 / total_chars as f32) > 0.18 {
        return true;
    }
    const MOJIBAKE_MARKERS: [&str; 12] = [
        "鈥", "銆", "锛", "鍙", "鏂", "寮", "缁", "鐮", "姝", "闂", "璇", "閿",
    ];
    MOJIBAKE_MARKERS.iter().any(|marker| trimmed.contains(marker))
}

fn is_question_mark_garbage(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return false;
    }
    let total_chars = trimmed.chars().count().max(1);
    let question_like = trimmed.chars().filter(|ch| matches!(ch, '?' | '？')).count();
    question_like >= 4 && (question_like as f32 / total_chars as f32) > 0.3
}

fn is_low_value_summary_text(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() || looks_like_corrupted_text(trimmed) {
        return true;
    }
    if trimmed.starts_with('[') && trimmed.contains("编码损坏") {
        return true;
    }

    const GENERIC_FALLBACKS: [&str; 14] = [
        "无法理解您发送的内容",
        "无法正常显示的内容",
        "您的输入仍然显示为无法识别的字符",
        "乱码字符",
        "请重新描述您的需求",
        "重新发送一条",
        "历史消息已因旧编码损坏而清洗",
        "原文无法恢复",
        "cannot understand your message",
        "unable to understand your message",
        "corrupted encoding",
        "garbled characters",
        "please resend",
        "unreadable content",
    ];

    GENERIC_FALLBACKS.iter().any(|needle| trimmed.contains(needle))
}

fn auto_title(messages: &[MessageBlock]) -> Option<String> {
    let user_msg = messages
        .iter()
        .find_map(|m| {
            if let MessageBlock::User { content, .. } = m {
                if is_low_value_summary_text(content) {
                    None
                } else {
                    Some(content.as_str())
                }
            } else {
                None
            }
        })
        .or_else(|| {
            messages.iter().find_map(|m| {
                if let MessageBlock::User { content, .. } = m {
                    Some(content.as_str())
                } else {
                    None
                }
            })
        })?;

    compact_message_text(user_msg, 28).or_else(|| Some("New conversation".to_string()))
}

fn auto_summary(messages: &[MessageBlock]) -> Option<String> {
    let preferred = messages.iter().rev().find_map(|message| match message {
        MessageBlock::Assistant { content }
        | MessageBlock::AssistantStreaming { content }
        | MessageBlock::Error { content }
        | MessageBlock::System { content } => {
            if is_low_value_summary_text(content) {
                None
            } else {
                compact_message_text(content, 42)
            }
        }
        _ => None,
    });

    if preferred.is_some() {
        return preferred;
    }

    messages.iter().rev().find_map(|message| match message {
        MessageBlock::User { content, .. } => {
            if is_low_value_summary_text(content) {
                None
            } else {
                compact_message_text(content, 42)
            }
        }
        _ => None,
    })
}

fn try_restore_mojibake(raw: &str) -> Option<String> {
    let mut candidates = Vec::new();

    if let Some(restored) = reinterpret_latin1_as_utf8(raw) {
        candidates.push(restored);
    }
    if let Some(restored) = reinterpret_windows_1252_as_utf8(raw) {
        candidates.push(restored);
    }

    candidates
        .into_iter()
        .filter(|candidate| !looks_like_corrupted_text(candidate))
        .filter(|candidate| candidate.chars().any(|ch| !matches!(ch, '?' | '？')))
        .max_by_key(|candidate| candidate.chars().filter(|ch| is_human_text_char(*ch)).count())
}

fn reinterpret_latin1_as_utf8(raw: &str) -> Option<String> {
    let bytes: Option<Vec<u8>> = raw
        .chars()
        .map(|ch| {
            let code = ch as u32;
            if code <= 0xff {
                Some(code as u8)
            } else {
                None
            }
        })
        .collect();
    let bytes = bytes?;
    String::from_utf8(bytes).ok()
}

fn reinterpret_windows_1252_as_utf8(raw: &str) -> Option<String> {
    let bytes: Option<Vec<u8>> = raw
        .chars()
        .map(windows_1252_byte)
        .collect();
    let bytes = bytes?;
    String::from_utf8(bytes).ok()
}

fn windows_1252_byte(ch: char) -> Option<u8> {
    let code = ch as u32;
    if code <= 0xff {
        return Some(code as u8);
    }
    Some(match ch {
        '€' => 0x80,
        '‚' => 0x82,
        'ƒ' => 0x83,
        '„' => 0x84,
        '…' => 0x85,
        '†' => 0x86,
        '‡' => 0x87,
        'ˆ' => 0x88,
        '‰' => 0x89,
        'Š' => 0x8A,
        '‹' => 0x8B,
        'Œ' => 0x8C,
        'Ž' => 0x8E,
        '‘' => 0x91,
        '’' => 0x92,
        '“' => 0x93,
        '”' => 0x94,
        '•' => 0x95,
        '–' => 0x96,
        '—' => 0x97,
        '˜' => 0x98,
        '™' => 0x99,
        'š' => 0x9A,
        '›' => 0x9B,
        'œ' => 0x9C,
        'ž' => 0x9E,
        'Ÿ' => 0x9F,
        _ => return None,
    })
}

fn is_human_text_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
        || ('\u{4e00}'..='\u{9fff}').contains(&ch)
        || ('\u{3040}'..='\u{30ff}').contains(&ch)
        || ('\u{ac00}'..='\u{d7af}').contains(&ch)
}
