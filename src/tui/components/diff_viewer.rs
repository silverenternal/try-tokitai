//! Diff viewer — Claude Code style code change display
//! Green for additions (+), red for deletions (-), dim for context

use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use serde::{Deserialize, Serialize};

use crate::text_encoding::read_text_file;

/// A single line in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffLine {
    Add(String),
    Remove(String),
    Context(String),
    Header(String),
}

/// Represents a complete diff between old and new file content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub file_path: String,
    pub lines: Vec<DiffLine>,
    pub added: usize,
    pub removed: usize,
    #[serde(default)]
    pub before_content: String,
    #[serde(default)]
    pub after_content: String,
}

impl FileDiff {
    /// Compute a simple line-by-line diff
    pub fn compute(file_path: &str, old: &str, new: &str) -> Self {
        let old_lines: Vec<&str> = old.lines().collect();
        let new_lines: Vec<&str> = new.lines().collect();

        let mut diff_lines = Vec::new();
        diff_lines.push(DiffLine::Header(format!("📄 {}", file_path)));

        let mut added = 0;
        let mut removed = 0;

        // Simple diff: find common prefix/suffix, show changed middle
        let mut prefix = 0;
        while prefix < old_lines.len() && prefix < new_lines.len()
            && old_lines[prefix] == new_lines[prefix]
        {
            if prefix < 3 {
                diff_lines.push(DiffLine::Context(format!(
                    "  {}",
                    old_lines[prefix]
                )));
            } else if prefix == 3 {
                diff_lines.push(DiffLine::Context("  ...".to_string()));
            }
            prefix += 1;
        }

        let mut suffix = 0;
        while suffix < old_lines.len().saturating_sub(prefix)
            && suffix < new_lines.len().saturating_sub(prefix)
            && old_lines[old_lines.len() - 1 - suffix]
                == new_lines[new_lines.len() - 1 - suffix]
        {
            suffix += 1;
        }

        // Changed section
        let old_changed = &old_lines[prefix..old_lines.len().saturating_sub(suffix)];
        let new_changed = &new_lines[prefix..new_lines.len().saturating_sub(suffix)];

        for line in old_changed {
            diff_lines.push(DiffLine::Remove(format!("- {}", line)));
            removed += 1;
        }
        for line in new_changed {
            diff_lines.push(DiffLine::Add(format!("+ {}", line)));
            added += 1;
        }

        // Suffix context
        let suffix_start = old_lines.len().saturating_sub(suffix);
        let skipped = suffix_start.saturating_sub(prefix + old_changed.len());
        if skipped > 3 && suffix > 0 {
            diff_lines.push(DiffLine::Context("  ...".to_string()));
        }
        for i in suffix_start..old_lines.len().min(suffix_start + 3) {
            if i < old_lines.len() {
                diff_lines
                    .push(DiffLine::Context(format!("  {}", old_lines[i])));
            }
        }

        Self {
            file_path: file_path.to_string(),
            lines: diff_lines,
            added,
            removed,
            before_content: old.to_string(),
            after_content: new.to_string(),
        }
    }

    /// Render the diff as ratatui Lines
    pub fn render(&self) -> Vec<Line<'static>> {
        let mut out = Vec::new();

        for line in &self.lines {
            match line {
                DiffLine::Header(text) => {
                    out.push(Line::from(Span::styled(
                        text.clone(),
                        Style::default().fg(Color::White),
                    )));
                }
                DiffLine::Add(text) => {
                    out.push(Line::from(Span::styled(
                        text.clone(),
                        Style::default()
                            .fg(Color::Green)
                            .bg(Color::Rgb(0, 40, 0)),
                    )));
                }
                DiffLine::Remove(text) => {
                    out.push(Line::from(Span::styled(
                        text.clone(),
                        Style::default()
                            .fg(Color::Red)
                            .bg(Color::Rgb(50, 0, 0)),
                    )));
                }
                DiffLine::Context(text) => {
                    out.push(Line::from(Span::styled(
                        text.clone(),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }
        }

        // Summary line
        if self.added > 0 || self.removed > 0 {
            out.push(Line::from(vec![
                Span::styled(
                    format!("  +{} ", self.added),
                    Style::default().fg(Color::Green),
                ),
                Span::styled(
                    format!("-{} ", self.removed),
                    Style::default().fg(Color::Red),
                ),
                Span::styled(
                    format!("in {}", self.file_path),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }

        out
    }
}

/// Detect if a write_file result represents a file modification
pub fn detect_file_write(tool_name: &str, args: &serde_json::Value, result: &str) -> Option<FileDiff> {
    if tool_name != "write_file" && tool_name != "edit_file" {
        return None;
    }

    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let new_content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");

    // Try to read old content for diff
    let old_content = read_text_file(std::path::Path::new(path)).unwrap_or_default();

    if old_content == new_content || new_content.is_empty() {
        return None;
    }

    Some(FileDiff::compute(path, &old_content, new_content))
}
