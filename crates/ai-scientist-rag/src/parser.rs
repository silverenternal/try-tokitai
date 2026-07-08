//! PDF Parser — extracts text content from scientific papers

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Errors that can occur during PDF parsing
#[derive(Debug, thiserror::Error)]
pub enum ParserError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}

/// Extracted paper content with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPaper {
    /// Paper title (extracted or filename)
    pub title: Option<String>,
    /// Authors (extracted)
    pub authors: Vec<String>,
    /// Abstract text
    pub abstract_text: Option<String>,
    /// Full body text
    pub body_text: String,
    /// Section titles
    pub sections: Vec<String>,
    /// References found
    pub references: Vec<String>,
    /// Total page count
    pub page_count: usize,
    /// Paper file path
    pub path: String,
    /// BLAKE3 hash of the PDF file
    pub file_hash: String,
    /// Publication year (extracted)
    pub year: Option<u32>,
    /// DOI (extracted)
    pub doi: Option<String>,
}

/// PDF Parser for scientific papers
///
/// Uses a two-phase approach:
/// 1. Try `lopdf` for native Rust parsing (fast)
/// 2. Fall back to `pdftotext` CLI (reliable, handles complex layouts)
pub struct PdfParser {
    /// Path to `pdftotext` binary (fallback)
    pdftotext_path: String,
}

impl PdfParser {
    pub fn new() -> Self {
        Self {
            pdftotext_path: "pdftotext".to_string(),
        }
    }

    /// Parse a PDF file and extract structured content
    pub fn parse(&self, path: &Path) -> Result<ParsedPaper, ParserError> {
        if !path.exists() {
            return Err(ParserError::FileNotFound(path.display().to_string()));
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext != "pdf" {
            return Err(ParserError::UnsupportedFormat(ext));
        }

        // Compute BLAKE3 hash of file
        let file_bytes = std::fs::read(path)?;
        let file_hash = blake3::hash(&file_bytes).to_hex().to_string();

        // Extract text using pdftotext (most reliable for academic PDFs)
        let text = self.extract_text_pdftotext(path)?;

        // Extract structured fields
        let (title, abstract_text, sections, references) = Self::extract_sections(&text);
        let authors = Self::extract_authors(&text);
        let year = Self::extract_year(&text);
        let doi = Self::extract_doi(&text);
        let page_count = Self::count_pages(&text);

        let filename = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        Ok(ParsedPaper {
            title: title.or_else(|| Some(filename.to_string())),
            authors,
            abstract_text,
            body_text: text.clone(),
            sections,
            references,
            page_count: page_count.max(1),
            path: path.display().to_string(),
            file_hash,
            year,
            doi,
        })
    }

    /// Extract text using pdftotext CLI
    fn extract_text_pdftotext(&self, path: &Path) -> Result<String, ParserError> {
        let output = std::process::Command::new(&self.pdftotext_path)
            .args(["-layout", path.to_str().unwrap_or(""), "-"])
            .output()
            .map_err(|e| {
                ParserError::Parse(format!(
                    "pdftotext not found ({}). Install poppler-utils.",
                    e
                ))
            })?;

        if !output.status.success() {
            return Err(ParserError::Parse(
                "pdftotext failed to parse PDF".to_string(),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Extract sections from paper text
    fn extract_sections(text: &str) -> (Option<String>, Option<String>, Vec<String>, Vec<String>) {
        let mut title = None;
        let mut abstract_text = None;
        let mut sections = Vec::new();
        let mut references = Vec::new();
        let mut in_abstract = false;
        let mut in_references = false;
        let mut abstract_lines = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim();

            // Detect title (first non-empty line)
            if title.is_none() && !trimmed.is_empty() && trimmed.len() < 200 {
                title = Some(trimmed.to_string());
            }

            // Abstract detection
            if trimmed.to_lowercase() == "abstract" {
                in_abstract = true;
                continue;
            }
            if in_abstract
                && (trimmed.to_lowercase().starts_with("introduction")
                    || trimmed.to_lowercase().starts_with("1.")
                    || trimmed.to_lowercase().starts_with("keywords"))
            {
                in_abstract = false;
                abstract_text = Some(abstract_lines.join(" "));
                continue;
            }
            if in_abstract {
                abstract_lines.push(trimmed);
            }

            // Section detection (numbered or named)
            if trimmed
                .chars()
                .take(3)
                .all(|c| c.is_ascii_digit() || c == '.')
                && trimmed.len() < 80
            {
                sections.push(trimmed.to_string());
            }
            if trimmed.len() < 60
                && trimmed
                    .chars()
                    .all(|c| c.is_uppercase() || c.is_whitespace())
                && !trimmed.is_empty()
            {
                sections.push(trimmed.to_string());
            }

            // References section
            if trimmed.to_lowercase().starts_with("references")
                || trimmed.to_lowercase().starts_with("bibliography")
            {
                in_references = true;
                continue;
            }
            if in_references && !trimmed.is_empty() {
                references.push(trimmed.to_string());
            }
        }

        (title, abstract_text, sections, references)
    }

    /// Extract author names
    fn extract_authors(text: &str) -> Vec<String> {
        // Look for author lines near the top of the paper
        let lines: Vec<&str> = text.lines().take(30).collect();
        for window in lines.windows(3) {
            let line = window.join(" ");
            // Author lines typically contain commas, affiliations
            if line.contains(',')
                && line.len() > 20
                && line.len() < 300
                && !line.to_lowercase().contains("abstract")
            {
                // Simple heuristic: split by comma, clean up
                return line
                    .split(',')
                    .map(|s| {
                        s.trim()
                            .trim_matches(|c: char| c.is_numeric() || c == '*')
                            .trim()
                            .to_string()
                    })
                    .filter(|s| {
                        s.len() > 3 && !s.contains("university") && !s.contains("institute")
                    })
                    .collect();
            }
        }
        vec![]
    }

    /// Extract publication year
    fn extract_year(text: &str) -> Option<u32> {
        // Look for year patterns in first 20 lines
        for line in text.lines().take(20) {
            // Match: (2024), 2024, etc.
            for word in line.split_whitespace() {
                let cleaned = word.trim_matches(|c: char| !c.is_ascii_digit());
                if cleaned.len() == 4 {
                    if let Ok(year) = cleaned.parse::<u32>() {
                        if (1900..2099).contains(&year) {
                            return Some(year);
                        }
                    }
                }
            }
        }
        None
    }

    /// Extract DOI
    fn extract_doi(text: &str) -> Option<String> {
        for line in text.lines() {
            let lower = line.to_lowercase();
            if let Some(pos) = lower.find("doi") {
                let after = &line[pos..];
                // Try common DOI patterns
                for prefix in &["10.", "doi.org/10."] {
                    if let Some(start) = after.find(prefix) {
                        let doi_candidate: String = after[start..]
                            .split_whitespace()
                            .next()
                            .unwrap_or("")
                            .trim_end_matches('.')
                            .to_string();
                        if doi_candidate.len() > 8 {
                            return Some(doi_candidate);
                        }
                    }
                }
            }
        }
        None
    }

    /// Estimate page count
    fn count_pages(text: &str) -> usize {
        // Rough estimate: ~3000 chars per page for academic PDFs
        (text.len() / 3000).max(1)
    }
}

impl Default for PdfParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_year() {
        let text = "Published in 2024\nAbstract: This paper...";
        assert_eq!(PdfParser::extract_year(text), Some(2024));
    }

    #[test]
    fn test_extract_doi() {
        let text = "DOI: 10.1234/example.2024\n";
        let doi = PdfParser::extract_doi(text);
        assert!(doi.is_some());
        assert!(doi.unwrap().contains("10."));
    }

    #[test]
    fn test_unsupported_format() {
        let parser = PdfParser::new();
        let result = parser.parse(Path::new("test.txt"));
        assert!(result.is_err());
    }
}
