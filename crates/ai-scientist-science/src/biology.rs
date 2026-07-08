//! Biology Tool Interface
//!
//! Trait-based abstraction for bioinformatics computations.
//! Includes a local implementation for common sequence operations so the
//! platform remains functional even when Biopython is unavailable.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

use crate::python_bridge::{find_python_with_module, run_python_json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiologyResult {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

/// Sequence types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SequenceType {
    Dna,
    Rna,
    Protein,
}

/// Biology Tool trait
#[async_trait::async_trait]
pub trait BiologyToolInterface: Send + Sync {
    /// Translate DNA -> protein
    async fn translate(&self, sequence: &str) -> Result<String, String>;

    /// Reverse complement of DNA
    async fn reverse_complement(&self, sequence: &str) -> Result<String, String>;

    /// Run BLAST search
    async fn blast(&self, sequence: &str, database: &str) -> Result<serde_json::Value, String>;

    /// Align two sequences (Needleman-Wunsch / Smith-Waterman)
    async fn align(&self, seq_a: &str, seq_b: &str) -> Result<serde_json::Value, String>;

    /// Calculate GC content
    async fn gc_content(&self, sequence: &str) -> Result<f64, String>;
}

/// Local biology implementation for sequence-level operations.
pub struct LocalBiologyTool;

impl LocalBiologyTool {
    fn normalize_dna(sequence: &str) -> Result<String, String> {
        let normalized = sequence
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| c.to_ascii_uppercase())
            .collect::<String>();

        if normalized.is_empty() {
            return Err("Sequence is empty".into());
        }

        if !normalized
            .chars()
            .all(|c| matches!(c, 'A' | 'C' | 'G' | 'T' | 'N'))
        {
            return Err("Sequence contains non-DNA characters".into());
        }

        Ok(normalized)
    }

    fn codon_table() -> HashMap<&'static str, char> {
        HashMap::from([
            ("TTT", 'F'),
            ("TTC", 'F'),
            ("TTA", 'L'),
            ("TTG", 'L'),
            ("CTT", 'L'),
            ("CTC", 'L'),
            ("CTA", 'L'),
            ("CTG", 'L'),
            ("ATT", 'I'),
            ("ATC", 'I'),
            ("ATA", 'I'),
            ("ATG", 'M'),
            ("GTT", 'V'),
            ("GTC", 'V'),
            ("GTA", 'V'),
            ("GTG", 'V'),
            ("TCT", 'S'),
            ("TCC", 'S'),
            ("TCA", 'S'),
            ("TCG", 'S'),
            ("CCT", 'P'),
            ("CCC", 'P'),
            ("CCA", 'P'),
            ("CCG", 'P'),
            ("ACT", 'T'),
            ("ACC", 'T'),
            ("ACA", 'T'),
            ("ACG", 'T'),
            ("GCT", 'A'),
            ("GCC", 'A'),
            ("GCA", 'A'),
            ("GCG", 'A'),
            ("TAT", 'Y'),
            ("TAC", 'Y'),
            ("TAA", '*'),
            ("TAG", '*'),
            ("CAT", 'H'),
            ("CAC", 'H'),
            ("CAA", 'Q'),
            ("CAG", 'Q'),
            ("AAT", 'N'),
            ("AAC", 'N'),
            ("AAA", 'K'),
            ("AAG", 'K'),
            ("GAT", 'D'),
            ("GAC", 'D'),
            ("GAA", 'E'),
            ("GAG", 'E'),
            ("TGT", 'C'),
            ("TGC", 'C'),
            ("TGA", '*'),
            ("TGG", 'W'),
            ("CGT", 'R'),
            ("CGC", 'R'),
            ("CGA", 'R'),
            ("CGG", 'R'),
            ("AGT", 'S'),
            ("AGC", 'S'),
            ("AGA", 'R'),
            ("AGG", 'R'),
            ("GGT", 'G'),
            ("GGC", 'G'),
            ("GGA", 'G'),
            ("GGG", 'G'),
        ])
    }

    fn reverse_complement_inner(sequence: &str) -> Result<String, String> {
        let dna = Self::normalize_dna(sequence)?;
        let reversed = dna
            .chars()
            .rev()
            .map(|c| match c {
                'A' => Ok('T'),
                'T' => Ok('A'),
                'C' => Ok('G'),
                'G' => Ok('C'),
                'N' => Ok('N'),
                _ => Err(format!("Unsupported nucleotide: {}", c)),
            })
            .collect::<Result<String, String>>()?;
        Ok(reversed)
    }

    fn translate_inner(sequence: &str) -> Result<String, String> {
        let dna = Self::normalize_dna(sequence)?;
        let table = Self::codon_table();
        let mut protein = String::new();

        for codon in dna.as_bytes().chunks(3) {
            if codon.len() < 3 {
                break;
            }

            let codon = std::str::from_utf8(codon).map_err(|e| e.to_string())?;
            let amino = table.get(codon).copied().unwrap_or('X');
            protein.push(amino);
        }

        Ok(protein)
    }

    fn align_inner(seq_a: &str, seq_b: &str) -> Result<serde_json::Value, String> {
        let a = seq_a.trim().to_ascii_uppercase();
        let b = seq_b.trim().to_ascii_uppercase();

        if a.is_empty() || b.is_empty() {
            return Err("Alignment requires two non-empty sequences".into());
        }

        let match_score = 1i32;
        let mismatch_score = -1i32;
        let gap_penalty = -1i32;

        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let mut score = vec![vec![0i32; b_chars.len() + 1]; a_chars.len() + 1];

        for i in 1..=a_chars.len() {
            score[i][0] = score[i - 1][0] + gap_penalty;
        }
        for j in 1..=b_chars.len() {
            score[0][j] = score[0][j - 1] + gap_penalty;
        }

        for i in 1..=a_chars.len() {
            for j in 1..=b_chars.len() {
                let diag = score[i - 1][j - 1]
                    + if a_chars[i - 1] == b_chars[j - 1] {
                        match_score
                    } else {
                        mismatch_score
                    };
                let up = score[i - 1][j] + gap_penalty;
                let left = score[i][j - 1] + gap_penalty;
                score[i][j] = diag.max(up).max(left);
            }
        }

        let mut aligned_a = String::new();
        let mut aligned_b = String::new();
        let mut i = a_chars.len();
        let mut j = b_chars.len();

        while i > 0 || j > 0 {
            if i > 0 && j > 0 {
                let diag_score = score[i - 1][j - 1]
                    + if a_chars[i - 1] == b_chars[j - 1] {
                        match_score
                    } else {
                        mismatch_score
                    };
                if score[i][j] == diag_score {
                    aligned_a.insert(0, a_chars[i - 1]);
                    aligned_b.insert(0, b_chars[j - 1]);
                    i -= 1;
                    j -= 1;
                    continue;
                }
            }

            if i > 0 && score[i][j] == score[i - 1][j] + gap_penalty {
                aligned_a.insert(0, a_chars[i - 1]);
                aligned_b.insert(0, '-');
                i -= 1;
            } else if j > 0 {
                aligned_a.insert(0, '-');
                aligned_b.insert(0, b_chars[j - 1]);
                j -= 1;
            }
        }

        let matches = aligned_a
            .chars()
            .zip(aligned_b.chars())
            .filter(|(left, right)| left == right)
            .count();
        let identity = matches as f64 / aligned_a.len().max(1) as f64;

        Ok(json!({
            "algorithm": "needleman_wunsch",
            "score": score[a_chars.len()][b_chars.len()],
            "aligned_a": aligned_a,
            "aligned_b": aligned_b,
            "identity": identity
        }))
    }
}

pub struct AutoBiologyTool;

impl AutoBiologyTool {
    fn python_backend_available() -> Option<String> {
        find_python_with_module("Bio")
    }

    fn biopython_translate(python: &str, sequence: &str) -> Result<String, String> {
        let script = r#"
import json
import sys
from Bio.Seq import Seq

sequence = sys.argv[1]
print(json.dumps(str(Seq(sequence).translate(to_stop=False))))
"#;
        run_python_json::<String>(python, script, &[sequence])
    }

    fn biopython_reverse_complement(python: &str, sequence: &str) -> Result<String, String> {
        let script = r#"
import json
import sys
from Bio.Seq import Seq

sequence = sys.argv[1]
print(json.dumps(str(Seq(sequence).reverse_complement())))
"#;
        run_python_json::<String>(python, script, &[sequence])
    }

    fn biopython_gc_content(python: &str, sequence: &str) -> Result<f64, String> {
        let script = r#"
import json
import sys

sequence = sys.argv[1].upper()
if not sequence:
    raise SystemExit("Sequence is empty")
gc = sum(1 for ch in sequence if ch in ("G", "C"))
print(json.dumps(gc / len(sequence)))
"#;
        run_python_json::<f64>(python, script, &[sequence])
    }

    fn biopython_align(
        python: &str,
        seq_a: &str,
        seq_b: &str,
    ) -> Result<serde_json::Value, String> {
        let script = r#"
import json
import sys
from Bio import pairwise2

seq_a = sys.argv[1]
seq_b = sys.argv[2]
alignments = pairwise2.align.globalms(seq_a, seq_b, 1, -1, -1, -1, one_alignment_only=True)
if not alignments:
    raise SystemExit("Alignment failed")
alignment = alignments[0]
matches = sum(1 for left, right in zip(alignment.seqA, alignment.seqB) if left == right)
identity = matches / max(len(alignment.seqA), 1)
payload = {
    "algorithm": "biopython_pairwise2_global",
    "score": alignment.score,
    "aligned_a": alignment.seqA,
    "aligned_b": alignment.seqB,
    "identity": identity
}
print(json.dumps(payload))
"#;
        run_python_json::<serde_json::Value>(python, script, &[seq_a, seq_b])
    }
}

#[async_trait::async_trait]
impl BiologyToolInterface for LocalBiologyTool {
    async fn translate(&self, sequence: &str) -> Result<String, String> {
        Self::translate_inner(sequence)
    }

    async fn reverse_complement(&self, sequence: &str) -> Result<String, String> {
        Self::reverse_complement_inner(sequence)
    }

    async fn blast(&self, sequence: &str, database: &str) -> Result<serde_json::Value, String> {
        let normalized = Self::normalize_dna(sequence)?;
        Ok(json!({
            "status": "unavailable",
            "backend": "local",
            "database": database,
            "query_length": normalized.len(),
            "message": "BLAST requires an external database/runtime. Local fallback only validates input."
        }))
    }

    async fn align(&self, seq_a: &str, seq_b: &str) -> Result<serde_json::Value, String> {
        Self::align_inner(seq_a, seq_b)
    }

    async fn gc_content(&self, sequence: &str) -> Result<f64, String> {
        let dna = Self::normalize_dna(sequence)?;
        let gc = dna.chars().filter(|c| matches!(c, 'G' | 'C')).count();
        Ok(gc as f64 / dna.len() as f64)
    }
}

#[async_trait::async_trait]
impl BiologyToolInterface for AutoBiologyTool {
    async fn translate(&self, sequence: &str) -> Result<String, String> {
        if let Some(python) = Self::python_backend_available() {
            return Self::biopython_translate(&python, sequence);
        }
        let local = LocalBiologyTool;
        local.translate(sequence).await
    }

    async fn reverse_complement(&self, sequence: &str) -> Result<String, String> {
        if let Some(python) = Self::python_backend_available() {
            return Self::biopython_reverse_complement(&python, sequence);
        }
        let local = LocalBiologyTool;
        local.reverse_complement(sequence).await
    }

    async fn blast(&self, sequence: &str, database: &str) -> Result<serde_json::Value, String> {
        let local = LocalBiologyTool;
        let mut result = local.blast(sequence, database).await?;
        if let Some(object) = result.as_object_mut() {
            let backend = if Self::python_backend_available().is_some() {
                "biopython_available_local_fallback"
            } else {
                "local"
            };
            object.insert("backend_mode".to_string(), json!(backend));
        }
        Ok(result)
    }

    async fn align(&self, seq_a: &str, seq_b: &str) -> Result<serde_json::Value, String> {
        if let Some(python) = Self::python_backend_available() {
            return Self::biopython_align(&python, seq_a, seq_b);
        }
        let local = LocalBiologyTool;
        local.align(seq_a, seq_b).await
    }

    async fn gc_content(&self, sequence: &str) -> Result<f64, String> {
        if let Some(python) = Self::python_backend_available() {
            return Self::biopython_gc_content(&python, sequence);
        }
        let local = LocalBiologyTool;
        local.gc_content(sequence).await
    }
}

/// Stub implementation
pub struct StubBiologyTool;

#[async_trait::async_trait]
impl BiologyToolInterface for StubBiologyTool {
    async fn translate(&self, _sequence: &str) -> Result<String, String> {
        Err("Biology tool not configured. Install Biopython.".into())
    }

    async fn reverse_complement(&self, _sequence: &str) -> Result<String, String> {
        Err("Biology tool not configured.".into())
    }

    async fn blast(&self, _sequence: &str, _database: &str) -> Result<serde_json::Value, String> {
        Err("Biology tool not configured.".into())
    }

    async fn align(&self, _seq_a: &str, _seq_b: &str) -> Result<serde_json::Value, String> {
        Err("Biology tool not configured.".into())
    }

    async fn gc_content(&self, _sequence: &str) -> Result<f64, String> {
        Err("Biology tool not configured.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::{BiologyToolInterface, LocalBiologyTool};

    #[tokio::test]
    async fn test_translate_and_gc_content() {
        let tool = LocalBiologyTool;
        let protein = tool.translate("ATGGCC").await.unwrap();
        assert_eq!(protein, "MA");

        let gc = tool.gc_content("ATGGCC").await.unwrap();
        assert!((gc - 4.0 / 6.0).abs() < 1e-9);
    }

    #[tokio::test]
    async fn test_reverse_complement_and_align() {
        let tool = LocalBiologyTool;
        let rc = tool.reverse_complement("ATGC").await.unwrap();
        assert_eq!(rc, "GCAT");

        let alignment = tool.align("GATTACA", "GCATGCU").await.unwrap();
        assert_eq!(alignment["algorithm"], "needleman_wunsch");
        assert!(alignment["score"].as_i64().unwrap() <= 7);
    }
}
