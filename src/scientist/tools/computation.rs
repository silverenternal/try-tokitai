//! Computation Tools — Python/R/Julia script execution with sandboxing

use serde_json::Value;
use std::process::Command;
use std::time::Duration;
use tokitai::tool;

pub struct ComputationTools;

/// Execute a subprocess command with timeout and capture all output.
/// Returns JSON with stdout, stderr, exit_code, and timed_out flag.
fn run_command_with_timeout(
    program: &str,
    args: &[&str],
    input: &str,
    timeout_secs: u64,
) -> Value {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return serde_json::json!({
                "status": "error",
                "error": format!("Failed to start {}: {}", program, e),
                "stdout": "",
                "stderr": "",
                "exit_code": -1,
                "timed_out": false
            });
        }
    };

    // Write stdin if any
    if !input.is_empty() {
        use std::io::Write;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input.as_bytes());
        }
    }

    // Wait with timeout
    let timeout = Duration::from_secs(timeout_secs);
    let result = match std::process::Child::try_wait(&mut child) {
        Ok(Some(status)) => Ok(status), // already exited
        _ => {
            // Wait with timeout
            let start = std::time::Instant::now();
            loop {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return serde_json::json!({
                        "status": "timeout",
                        "error": format!("Execution timed out after {} seconds", timeout_secs),
                        "stdout": "",
                        "stderr": "",
                        "exit_code": -1,
                        "timed_out": true
                    });
                }
                match child.try_wait() {
                    Ok(Some(status)) => break Ok(status),
                    Ok(None) => {
                        std::thread::sleep(Duration::from_millis(50));
                        continue;
                    }
                    Err(e) => break Err(e),
                }
            }
        }
    };

    let output = child.wait_with_output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let exit_code = out.status.code().unwrap_or(-1);

            if out.status.success() {
                serde_json::json!({
                    "status": "success",
                    "stdout": stdout,
                    "stderr": stderr,
                    "exit_code": exit_code,
                    "timed_out": false
                })
            } else {
                serde_json::json!({
                    "status": "error",
                    "error": format!("Process exited with code {}", exit_code),
                    "stdout": stdout,
                    "stderr": stderr,
                    "exit_code": exit_code,
                    "timed_out": false
                })
            }
        }
        Err(e) => serde_json::json!({
            "status": "error",
            "error": format!("Failed to get process output: {}", e),
            "stdout": "",
            "stderr": "",
            "exit_code": -1,
            "timed_out": false
        }),
    }
}

/// Find available Python interpreter: try "python" then "python3"
fn find_python() -> &'static str {
    if Command::new("python")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        "python"
    } else {
        "python3"
    }
}

#[tool]
impl ComputationTools {
    /// Execute a Python script with timeout and return structured results.
    ///
    /// ## Parameters
    /// - `code`: Python code to execute
    /// - `timeout_secs`: Max execution time in seconds (default 30, max 120)
    ///
    /// ## Returns
    /// JSON with stdout, stderr, exit_code, and timed_out flag.
    ///
    /// ## Security
    /// Code is executed in a subprocess. For production, use Docker sandboxing.
    pub fn run_python(&self, code: String, timeout_secs: Option<u64>) -> Result<Value, String> {
        let timeout = timeout_secs.unwrap_or(30).min(120);
        let python = find_python();

        let result = run_command_with_timeout(python, &["-c", &code], "", timeout);

        Ok(serde_json::json!({
            "operation": "run_python",
            "python": python,
            "timeout_secs": timeout,
            "result": result
        }))
    }

    /// Execute R code and return results.
    pub fn run_r(&self, code: String, packages: Option<Vec<String>>) -> Result<Value, String> {
        let timeout = 60u64;

        // Load requested packages
        let preamble = if let Some(pkgs) = packages {
            pkgs.iter()
                .map(|p| format!("library({})", p))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            String::new()
        };

        let full_code = format!("{}\n{}", preamble, code);
        let result = run_command_with_timeout("Rscript", &["-e", &full_code], "", timeout);

        Ok(serde_json::json!({
            "operation": "run_r",
            "timeout_secs": timeout,
            "result": result
        }))
    }

    /// Execute Julia code and return results.
    pub fn run_julia(&self, code: String) -> Result<Value, String> {
        let timeout = 60u64;
        let result = run_command_with_timeout("julia", &["-e", &code], "", timeout);

        Ok(serde_json::json!({
            "operation": "run_julia",
            "timeout_secs": timeout,
            "result": result
        }))
    }
}
