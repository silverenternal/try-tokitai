//! Computation tools for lightweight, reproducible experiment execution.

use crate::text_encoding::decode_bytes;
use crate::toolchain::{default_toolchain_command, detect_toolchain_executable};
use serde_json::Value;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tokitai::tool;

pub struct ComputationTools;

fn run_command_with_timeout(program: &str, args: &[&str], input: &str, timeout_secs: u64) -> Value {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(err) => {
            return serde_json::json!({
                "status": "error",
                "error": format!("Failed to start {}: {}", program, err),
                "stdout": "",
                "stderr": "",
                "exit_code": -1,
                "timed_out": false
            });
        }
    };

    if !input.is_empty() {
        use std::io::Write;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input.as_bytes());
        }
    }

    let timeout = Duration::from_secs(timeout_secs);
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
            Ok(Some(_)) => break,
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(err) => {
                return serde_json::json!({
                    "status": "error",
                    "error": format!("Failed while waiting for process: {}", err),
                    "stdout": "",
                    "stderr": "",
                    "exit_code": -1,
                    "timed_out": false
                });
            }
        }
    }

    match child.wait_with_output() {
        Ok(output) => {
            let stdout = decode_bytes(&output.stdout);
            let stderr = decode_bytes(&output.stderr);
            let exit_code = output.status.code().unwrap_or(-1);
            if output.status.success() {
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
        Err(err) => serde_json::json!({
            "status": "error",
            "error": format!("Failed to collect process output: {}", err),
            "stdout": "",
            "stderr": "",
            "exit_code": -1,
            "timed_out": false
        }),
    }
}

fn detect_runtime_command(key: &str) -> Option<String> {
    detect_toolchain_executable(key).or_else(|| {
        let fallback = default_toolchain_command(key);
        if Command::new(&fallback)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            Some(fallback)
        } else {
            None
        }
    })
}

fn find_python() -> Option<String> {
    detect_runtime_command("python")
}

#[tool]
impl ComputationTools {
    /// Execute inline Python code.
    pub fn run_python(&self, code: String, timeout_secs: Option<u64>) -> Result<Value, String> {
        let timeout = timeout_secs.unwrap_or(30).min(120);
        let python = find_python()
            .ok_or_else(|| "run_python: no working Python interpreter was found".to_string())?;
        let result = run_command_with_timeout(&python, &["-c", &code], "", timeout);
        Ok(serde_json::json!({
            "operation": "run_python",
            "python": python,
            "timeout_secs": timeout,
            "result": result
        }))
    }

    /// Execute a Python file directly from the workspace.
    pub fn run_python_file(
        &self,
        path: String,
        args_json: Option<String>,
        timeout_secs: Option<u64>,
    ) -> Result<Value, String> {
        let timeout = timeout_secs.unwrap_or(120).min(1800);
        let python = find_python().ok_or_else(|| {
            "run_python_file: no working Python interpreter was found".to_string()
        })?;
        let script_path = Path::new(&path);
        if !script_path.exists() {
            return Err(format!("run_python_file: file does not exist: {}", path));
        }
        if !script_path.is_file() {
            return Err(format!("run_python_file: path is not a file: {}", path));
        }

        let owned_args = args_json
            .as_deref()
            .map(|raw| serde_json::from_str::<Vec<String>>(raw).unwrap_or_default())
            .unwrap_or_default();
        let mut final_args = Vec::with_capacity(1 + owned_args.len());
        final_args.push(path.as_str());
        for item in &owned_args {
            final_args.push(item.as_str());
        }

        let result = run_command_with_timeout(&python, &final_args, "", timeout);
        Ok(serde_json::json!({
            "operation": "run_python_file",
            "python": python,
            "path": path,
            "args_json": args_json,
            "args": owned_args,
            "timeout_secs": timeout,
            "result": result
        }))
    }

    /// Execute R code when Rscript is available.
    pub fn run_r(&self, code: String, packages: Option<Vec<String>>) -> Result<Value, String> {
        let rscript = detect_runtime_command("rscript")
            .ok_or_else(|| "run_r: no working Rscript interpreter was found".to_string())?;
        let preamble = packages
            .unwrap_or_default()
            .iter()
            .map(|pkg| format!("library({})", pkg))
            .collect::<Vec<_>>()
            .join("\n");
        let full_code = if preamble.is_empty() {
            code
        } else {
            format!("{}\n{}", preamble, code)
        };
        let result = run_command_with_timeout(&rscript, &["-e", &full_code], "", 60);
        Ok(serde_json::json!({
            "operation": "run_r",
            "rscript": rscript,
            "timeout_secs": 60,
            "result": result
        }))
    }

    /// Execute Julia code when Julia is available.
    pub fn run_julia(&self, code: String) -> Result<Value, String> {
        let julia = detect_runtime_command("julia")
            .ok_or_else(|| "run_julia: no working Julia interpreter was found".to_string())?;
        let result = run_command_with_timeout(&julia, &["-e", &code], "", 60);
        Ok(serde_json::json!({
            "operation": "run_julia",
            "julia": julia,
            "timeout_secs": 60,
            "result": result
        }))
    }
}
