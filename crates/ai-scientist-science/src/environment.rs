//! Environment detection for scientific backends.

use std::process::Command;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendStatus {
    pub name: String,
    pub available: bool,
    pub mode: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEnvironmentReport {
    pub python: BackendStatus,
    pub rdkit: BackendStatus,
    pub biopython: BackendStatus,
    pub ase: BackendStatus,
    pub lammps: BackendStatus,
    pub openfoam: BackendStatus,
    pub lean: BackendStatus,
    pub mathlib: BackendStatus,
    pub psi4: BackendStatus,
    pub quantum_espresso: BackendStatus,
}

fn command_exists(cmd: &str, version_arg: &str) -> bool {
    Command::new(cmd)
        .arg(version_arg)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn find_python() -> Option<String> {
    for candidate in ["python", "python3"] {
        if command_exists(candidate, "--version") {
            return Some(candidate.to_string());
        }
    }
    None
}

fn python_module_status(name: &str, module: &str) -> BackendStatus {
    if let Some(python) = find_python() {
        let probe = format!("import {}", module);
        let available = Command::new(&python)
            .args(["-c", &probe])
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false);

        BackendStatus {
            name: name.to_string(),
            available,
            mode: if available { "python_sdk" } else { "local_fallback" }.to_string(),
            detail: if available {
                format!("Python module '{}' is importable via {}", module, python)
            } else {
                format!("Python module '{}' is not available; local fallback will be used", module)
            },
        }
    } else {
        BackendStatus {
            name: name.to_string(),
            available: false,
            mode: "local_fallback".to_string(),
            detail: "Python runtime not found".to_string(),
        }
    }
}

fn cli_status(name: &str, commands: &[(&str, &str)]) -> BackendStatus {
    for (command, version_arg) in commands {
        if command_exists(command, version_arg) {
            return BackendStatus {
                name: name.to_string(),
                available: true,
                mode: "cli_backend".to_string(),
                detail: format!("Detected command '{} {}'", command, version_arg),
            };
        }
    }

    BackendStatus {
        name: name.to_string(),
        available: false,
        mode: "local_fallback".to_string(),
        detail: "No known command detected".to_string(),
    }
}

fn lean_status() -> BackendStatus {
    let lean = command_exists("lean", "--version");
    let lake = command_exists("lake", "--version");

    if lean || lake {
        BackendStatus {
            name: "lean".to_string(),
            available: true,
            mode: "lean_runtime".to_string(),
            detail: if lean && lake {
                "Detected Lean and Lake commands".to_string()
            } else if lean {
                "Detected Lean command".to_string()
            } else {
                "Detected Lake command".to_string()
            },
        }
    } else {
        BackendStatus {
            name: "lean".to_string(),
            available: false,
            mode: "missing".to_string(),
            detail: "Lean runtime not detected".to_string(),
        }
    }
}

fn mathlib_status() -> BackendStatus {
    let cwd = std::env::current_dir().ok();
    let has_mathlib = cwd.as_ref().map(|dir| {
        [
            ".lake/packages/mathlib",
            ".lake/packages/Mathlib",
            "Mathlib",
        ]
        .iter()
        .any(|candidate| dir.join(candidate).exists())
    }).unwrap_or(false);

    let has_lean_project_files = cwd.as_ref().map(|dir| {
        ["lean-toolchain", "lakefile.lean", "lakefile.toml"]
            .iter()
            .any(|candidate| dir.join(candidate).exists())
    }).unwrap_or(false);

    BackendStatus {
        name: "mathlib".to_string(),
        available: has_mathlib,
        mode: if has_mathlib {
            "mathlib_workspace".to_string()
        } else if has_lean_project_files {
            "lean_project_without_mathlib".to_string()
        } else {
            "missing".to_string()
        },
        detail: if has_mathlib {
            "Detected a local Mathlib checkout in the current workspace".to_string()
        } else if has_lean_project_files {
            "Lean project files detected, but Mathlib was not found under .lake/packages".to_string()
        } else {
            "No local Lean project or Mathlib checkout detected".to_string()
        },
    }
}

pub fn detect_domain_environment() -> DomainEnvironmentReport {
    let python = if let Some(python) = find_python() {
        BackendStatus {
            name: "python".to_string(),
            available: true,
            mode: "runtime".to_string(),
            detail: format!("Detected Python runtime '{}'", python),
        }
    } else {
        BackendStatus {
            name: "python".to_string(),
            available: false,
            mode: "missing".to_string(),
            detail: "Python runtime not detected".to_string(),
        }
    };

    DomainEnvironmentReport {
        python,
        rdkit: python_module_status("rdkit", "rdkit"),
        biopython: python_module_status("biopython", "Bio"),
        ase: python_module_status("ase", "ase"),
        lammps: cli_status("lammps", &[("lmp", "-help"), ("lammps", "-help")]),
        openfoam: cli_status("openfoam", &[("simpleFoam", "-help"), ("foamExec", "-help")]),
        lean: lean_status(),
        mathlib: mathlib_status(),
        psi4: cli_status("psi4", &[("psi4", "--version")]),
        quantum_espresso: cli_status("quantum_espresso", &[("pw.x", "-h"), ("ph.x", "-h")]),
    }
}

#[cfg(test)]
mod tests {
    use super::detect_domain_environment;

    #[test]
    fn test_detect_domain_environment_returns_report() {
        let report = detect_domain_environment();
        assert_eq!(report.rdkit.name, "rdkit");
        assert_eq!(report.biopython.name, "biopython");
        assert_eq!(report.lammps.name, "lammps");
        assert_eq!(report.lean.name, "lean");
        assert_eq!(report.mathlib.name, "mathlib");
    }
}
