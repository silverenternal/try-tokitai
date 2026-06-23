use std::borrow::Cow;
use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use serde_json::Value;

pub fn decode_bytes(bytes: &[u8]) -> String {
    let bytes = strip_utf8_bom(bytes);

    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
        return repair_mojibake_if_needed(text);
    }

    #[cfg(windows)]
    {
        if let Some(text) = decode_windows_ansi(bytes) {
            return repair_mojibake_if_needed(text);
        }
    }

    repair_mojibake_if_needed(String::from_utf8_lossy(bytes).into_owned())
}

pub fn decode_bytes_lossy(bytes: &[u8]) -> Cow<'_, str> {
    Cow::Owned(decode_bytes(bytes))
}

pub fn read_text_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read '{}'", path.display()))?;
    Ok(decode_bytes(&bytes))
}

fn strip_utf8_bom(bytes: &[u8]) -> &[u8] {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        bytes
    }
}

#[cfg(windows)]
fn decode_windows_ansi(bytes: &[u8]) -> Option<String> {
    decode_with_code_page(bytes, 936)
        .filter(|text| looks_like_human_text(text))
        .or_else(|| decode_with_code_page(bytes, 54936).filter(|text| looks_like_human_text(text)))
}

#[cfg(windows)]
fn decode_with_code_page(bytes: &[u8], code_page: u32) -> Option<String> {
    if bytes.is_empty() {
        return Some(String::new());
    }

    #[link(name = "Kernel32")]
    extern "system" {
        fn MultiByteToWideChar(
            CodePage: u32,
            dwFlags: u32,
            lpMultiByteStr: *const u8,
            cbMultiByte: i32,
            lpWideCharStr: *mut u16,
            cchWideChar: i32,
        ) -> i32;
    }

    const MB_ERR_INVALID_CHARS: u32 = 0x0000_0008;
    let len = unsafe {
        MultiByteToWideChar(
            code_page,
            MB_ERR_INVALID_CHARS,
            bytes.as_ptr(),
            bytes.len() as i32,
            std::ptr::null_mut(),
            0,
        )
    };
    if len <= 0 {
        return None;
    }

    let mut wide = vec![0u16; len as usize];
    let written = unsafe {
        MultiByteToWideChar(
            code_page,
            MB_ERR_INVALID_CHARS,
            bytes.as_ptr(),
            bytes.len() as i32,
            wide.as_mut_ptr(),
            len,
        )
    };
    if written <= 0 {
        return None;
    }

    String::from_utf16(&wide[..written as usize]).ok()
}

fn repair_mojibake_if_needed(text: String) -> String {
    if let Some(candidate) = try_restore_mojibake(&text) {
        if should_prefer_restored(&text, &candidate) {
            return candidate;
        }
    }
    text
}

fn looks_like_corrupted_text(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return false;
    }

    let suspicious = [
        "锟",
        "�",
        "鈥",
        "馃",
        "鎴",
        "鏂",
        "寮",
        "璇",
        "杩",
        "鍙",
        "鐨",
        "缁",
        "浠",
        "锛",
        "銆",
    ];

    let suspicious_hits = suspicious
        .iter()
        .filter(|needle| trimmed.contains(**needle))
        .count();

    suspicious_hits >= 2 || is_question_mark_garbage(trimmed) || looks_like_latin1_utf8_mojibake(trimmed)
}

fn is_question_mark_garbage(raw: &str) -> bool {
    let total_chars = raw.chars().count().max(1);
    let question_like = raw.chars().filter(|ch| matches!(ch, '?' | '锛')).count();
    question_like >= 4 && (question_like as f32 / total_chars as f32) > 0.3
}

fn looks_like_human_text(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return true;
    }
    let humanish = trimmed
        .chars()
        .filter(|ch| is_human_text_char(*ch) || ch.is_ascii_punctuation() || ch.is_whitespace())
        .count();
    humanish * 10 >= trimmed.chars().count().max(1) * 6
}

fn should_prefer_restored(original: &str, candidate: &str) -> bool {
    if candidate == original || looks_like_corrupted_text(candidate) {
        return false;
    }

    let original_score = text_quality_score(original);
    let candidate_score = text_quality_score(candidate);
    let original_cjk = count_cjk_chars(original);
    let candidate_cjk = count_cjk_chars(candidate);

    if looks_like_corrupted_text(original) {
        return candidate_score >= original_score;
    }

    if candidate_cjk > original_cjk {
        return true;
    }

    looks_like_latin1_utf8_mojibake(original) && candidate_score >= original_score + 4
}

fn text_quality_score(raw: &str) -> i32 {
    raw.chars()
        .map(|ch| {
            if is_human_text_char(ch) {
                3
            } else if ch.is_ascii_punctuation() || ch.is_whitespace() {
                1
            } else {
                -2
            }
        })
        .sum()
}

fn count_cjk_chars(raw: &str) -> usize {
    raw.chars()
        .filter(|ch| ('\u{4e00}'..='\u{9fff}').contains(ch) || ('\u{3400}'..='\u{4dbf}').contains(ch))
        .count()
}

fn looks_like_latin1_utf8_mojibake(raw: &str) -> bool {
    let total = raw.chars().count().max(1);
    let suspicious = raw
        .chars()
        .filter(|ch| {
            matches!(
                ch,
                '\u{00C0}'..='\u{00FF}'
                    | '\u{0080}'..='\u{009F}'
                    | 'Ā' | 'Ă' | 'ą' | '€' | '™' | 'œ' | 'Œ' | 'š' | 'ž' | 'Ÿ'
            )
        })
        .count();
    suspicious >= 3 && suspicious * 10 >= total
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
        .filter(|candidate| candidate.chars().any(|ch| !matches!(ch, '?' | '锛')))
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
    let bytes: Option<Vec<u8>> = raw.chars().map(windows_1252_byte).collect();
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
        || ('\u{3400}'..='\u{4dbf}').contains(&ch)
        || ('\u{3040}'..='\u{30ff}').contains(&ch)
        || ('\u{ac00}'..='\u{d7af}').contains(&ch)
}

pub fn ensure_json_text(path: &Path) -> Result<String> {
    let content = read_text_file(path)?;
    if content.trim().is_empty() {
        return Err(anyhow!("decoded text is empty: {}", path.display()));
    }
    Ok(content)
}

pub fn normalize_text_for_display(raw: &str) -> String {
    repair_mojibake_if_needed(raw.replace('\u{feff}', ""))
}

pub fn normalize_json_strings(value: &mut Value) {
    match value {
        Value::String(text) => {
            *text = normalize_text_for_display(text);
        }
        Value::Array(items) => {
            for item in items {
                normalize_json_strings(item);
            }
        }
        Value::Object(map) => {
            for value in map.values_mut() {
                normalize_json_strings(value);
            }
        }
        _ => {}
    }
}
