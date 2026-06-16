//! Lean4 Formal Verification Tool
//!
//! Wraps Lean4 theorem prover via `lake env lean` for formal verification.
//! Verifies theorems, lemmas, and proofs written in Lean4.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanVerificationResult {
    pub success: bool,
    pub errors: Vec<LeanError>,
    pub warnings: Vec<String>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanError {
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub message: String,
    pub severity: String, // "error" | "warning" | "info"
}

pub struct LeanVerifier {
    lake_path: String,
    work_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanEnvironmentStatus {
    pub lean_path: Option<String>,
    pub lake_path: Option<String>,
    pub mathlib_present: bool,
    pub work_dir_exists: bool,
    pub configured: bool,
}

impl LeanVerifier {
    pub fn new(lake_path: impl Into<String>, work_dir: impl Into<PathBuf>) -> Self {
        Self {
            lake_path: lake_path.into(),
            work_dir: work_dir.into(),
        }
    }

    pub fn environment_status(&self) -> LeanEnvironmentStatus {
        let lean_path = Self::find_command("lean");
        let lake_path = Self::find_command(&self.lake_path);
        let work_dir_exists = self.work_dir.exists();
        let mathlib_present = self.work_dir.join("lean-toolchain").exists()
            || self.work_dir.join("lakefile.lean").exists()
            || self.work_dir.join("lakefile.toml").exists();
        let configured = work_dir_exists && lake_path.is_some();

        LeanEnvironmentStatus {
            lean_path,
            lake_path,
            mathlib_present,
            work_dir_exists,
            configured,
        }
    }

    /// Verify Lean4 code by writing to temp file and running `lake env lean`
    pub fn verify(&self, code: &str) -> LeanVerificationResult {
        let start = std::time::Instant::now();

        // Write code to temp file
        let temp_file = self.work_dir.join("_verify_temp.lean");
        if let Err(e) = std::fs::write(&temp_file, code) {
            return LeanVerificationResult {
                success: false,
                errors: vec![LeanError {
                    line: None,
                    column: None,
                    message: format!("Failed to write temp file: {}", e),
                    severity: "error".into(),
                }],
                warnings: vec![],
                elapsed_ms: start.elapsed().as_millis() as u64,
            };
        }

        // Run lake env lean
        let result = std::process::Command::new(&self.lake_path)
            .args(["env", "lean", temp_file.to_str().unwrap_or("_verify_temp.lean")])
            .current_dir(&self.work_dir)
            .output();

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_file);

        let elapsed = start.elapsed().as_millis() as u64;

        match result {
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);

                let errors = Self::parse_lean_errors(&stderr);
                let success = output.status.success() && errors.is_empty();

                LeanVerificationResult {
                    success,
                    errors,
                    warnings: Self::parse_lean_warnings(&stdout),
                    elapsed_ms: elapsed,
                }
            }
            Err(e) => LeanVerificationResult {
                success: false,
                errors: vec![LeanError {
                    line: None,
                    column: None,
                    message: format!("Lean execution failed: {}", e),
                    severity: "error".into(),
                }],
                warnings: vec![],
                elapsed_ms: elapsed,
            },
        }
    }

    /// Verify a theorem with explicit statement and proof
    pub fn verify_theorem(
        &self,
        name: &str,
        statement: &str,
        proof: &str,
    ) -> LeanVerificationResult {
        let code = format!(
            r#"import Mathlib

theorem {} : {} :=
by
  {}
"#,
            name, statement, proof
        );
        self.verify(&code)
    }

    /// Verify a lemma
    pub fn verify_lemma(
        &self,
        name: &str,
        statement: &str,
        proof: &str,
    ) -> LeanVerificationResult {
        let code = format!(
            r#"import Mathlib

lemma {} : {} :=
by
  {}
"#,
            name, statement, proof
        );
        self.verify(&code)
    }

    fn find_command(cmd: &str) -> Option<String> {
        std::process::Command::new(cmd)
            .arg("--version")
            .output()
            .ok()
            .and_then(|out| out.status.success().then(|| cmd.to_string()))
    }

    /// Parse Lean error output into structured errors
    fn parse_lean_errors(stderr: &str) -> Vec<LeanError> {
        let mut errors = Vec::new();

        for line in stderr.lines() {
            let trimmed = line.trim();

            // Lean errors: "error: message" or "file.lean:line:col: error: message"
            if trimmed.contains("error:") || trimmed.contains("error :") {
                // Try to parse line:col format
                let (line_num, col_num, msg) = if let Some(_colon_pos) = trimmed.find(':') {
                    let parts: Vec<&str> = trimmed.splitn(4, ':').collect();
                    if parts.len() >= 4 {
                        let l = parts[1].trim().parse::<usize>().ok();
                        let c = parts[2].trim().parse::<usize>().ok();
                        let m = parts[3..].join(":").trim().to_string();
                        (l, c, m)
                    } else {
                        (None, None, trimmed.to_string())
                    }
                } else {
                    (None, None, trimmed.to_string())
                };

                errors.push(LeanError {
                    line: line_num,
                    column: col_num,
                    message: msg,
                    severity: "error".into(),
                });
            }
        }

        errors
    }

    fn parse_lean_warnings(stdout: &str) -> Vec<String> {
        stdout
            .lines()
            .filter(|l| l.contains("warning:"))
            .map(|l| l.to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lean_errors() {
        let stderr = "_verify_temp.lean:5:10: error: type mismatch\n  expected: Nat\n  given: String";
        let errors = LeanVerifier::parse_lean_errors(stderr);
        assert!(!errors.is_empty());
        assert_eq!(errors[0].line, Some(5));
        assert_eq!(errors[0].column, Some(10));
        assert!(errors[0].message.contains("type mismatch"));
    }

    #[test]
    fn test_verify_theorem_format() {
        // This test won't actually run lean, just tests the code generation
        let code = format!(
            r#"import Mathlib

theorem {} : {} :=
by
  {}
"#,
            "my_theorem", "1 + 1 = 2", "rfl"
        );
        assert!(code.contains("theorem my_theorem"));
        assert!(code.contains("1 + 1 = 2"));
        assert!(code.contains("rfl"));
    }

    #[test]
    fn test_result_serialization() {
        let result = LeanVerificationResult {
            success: true,
            errors: vec![],
            warnings: vec![],
            elapsed_ms: 42,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"elapsed_ms\":42"));
    }

    #[test]
    fn test_environment_status_serialization() {
        let verifier = LeanVerifier::new("lake", ".");
        let status = verifier.environment_status();
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("mathlib_present"));
    }
}
