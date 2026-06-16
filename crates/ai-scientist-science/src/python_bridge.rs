//! Shared Python bridge for optional scientific SDK backends.

use std::process::Command;

use serde::de::DeserializeOwned;

pub fn find_python_with_module(module: &str) -> Option<String> {
    for candidate in ["python", "python3"] {
        let ok = Command::new(candidate)
            .args(["-c", &format!("import {}", module)])
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false);
        if ok {
            return Some(candidate.to_string());
        }
    }
    None
}

pub fn run_python_json<T: DeserializeOwned>(python: &str, script: &str, args: &[&str]) -> Result<T, String> {
    let output = Command::new(python)
        .arg("-c")
        .arg(script)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute {}: {}", python, e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    serde_json::from_slice::<T>(&output.stdout).map_err(|e| {
        format!(
            "Failed to parse Python JSON output: {}. Raw output: {}",
            e,
            String::from_utf8_lossy(&output.stdout)
        )
    })
}
