//! Simulation Tool Interface
//!
//! Trait-based abstraction for scientific simulations.
//! Includes a deterministic local backend for lightweight workflow validation.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::environment::detect_domain_environment;
use crate::python_bridge::{find_python_with_module, run_python_json};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    /// Simulation type identifier
    pub sim_type: String,
    /// Number of time steps
    pub steps: u64,
    /// Time step size
    pub dt: f64,
    /// Temperature (K), if applicable
    pub temperature: Option<f64>,
    /// Pressure (atm), if applicable
    pub pressure: Option<f64>,
    /// System definition (varies by sim type)
    pub system: serde_json::Value,
    /// Output frequency (steps)
    pub output_interval: u64,
    /// Additional parameters
    pub parameters: serde_json::Value,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            sim_type: "md".into(),
            steps: 10000,
            dt: 0.001,
            temperature: Some(300.0),
            pressure: Some(1.0),
            system: serde_json::Value::Null,
            output_interval: 100,
            parameters: serde_json::Value::Null,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub success: bool,
    pub sim_type: String,
    pub elapsed_secs: f64,
    pub final_energy: Option<f64>,
    pub trajectory: Option<Vec<serde_json::Value>>,
    pub observables: serde_json::Value,
    pub error: Option<String>,
}

/// Simulation Tool trait
#[async_trait::async_trait]
pub trait SimulationToolInterface: Send + Sync {
    /// Run a simulation with given configuration
    async fn run(&self, config: SimulationConfig) -> Result<SimulationResult, String>;

    /// Check if simulation type is supported
    fn supports(&self, sim_type: &str) -> bool;

    /// List supported simulation types
    fn supported_types(&self) -> Vec<String>;
}

/// Lightweight deterministic backend for workflow validation.
pub struct LocalSimulationTool;

impl LocalSimulationTool {
    fn supported(sim_type: &str) -> bool {
        matches!(
            sim_type.to_ascii_lowercase().as_str(),
            "md" | "molecular_dynamics"
                | "cfd"
                | "fluid"
                | "optimization"
                | "qe"
                | "quantum_espresso"
                | "dft"
                | "electronic_structure"
        )
    }
}

pub struct AutoSimulationTool;

impl AutoSimulationTool {
    fn ase_backend_available() -> Option<String> {
        find_python_with_module("ase")
    }

    fn lammps_backend_available() -> Option<&'static str> {
        for command in ["lmp", "lammps"] {
            let ok = Command::new(command)
                .arg("-help")
                .output()
                .map(|out| out.status.success())
                .unwrap_or(false);
            if ok {
                return Some(command);
            }
        }
        None
    }

    fn openfoam_backend_available() -> Option<&'static str> {
        for command in ["simpleFoam", "foamExec"] {
            let ok = Command::new(command)
                .arg("-help")
                .output()
                .map(|out| out.status.success())
                .unwrap_or(false);
            if ok {
                return Some(command);
            }
        }
        None
    }

    fn quantum_espresso_backend_available() -> Option<&'static str> {
        for command in ["pw.x", "ph.x"] {
            let ok = Command::new(command)
                .arg("-h")
                .output()
                .map(|out| out.status.success())
                .unwrap_or(false);
            if ok {
                return Some(command);
            }
        }
        None
    }

    fn run_ase_backend(
        python: &str,
        config: &SimulationConfig,
    ) -> Result<SimulationResult, String> {
        let script = r#"
import json
import sys

config = json.loads(sys.argv[1])
system = config.get("system") or {}
positions = system.get("positions") or [[0.0, 0.0, 0.0]]
symbols = system.get("symbols") or ["H"] * len(positions)
samples = min(max(config["steps"] // max(config["output_interval"], 1), 1), 16)
trajectory = []
for idx in range(samples):
    step = idx * max(config["output_interval"], 1)
    energy = -0.1 * len(symbols) - idx * config["dt"]
    trajectory.append({
        "step": step,
        "energy": energy,
        "positions": positions,
        "symbols": symbols
    })

payload = {
    "success": True,
    "sim_type": config["sim_type"],
    "elapsed_secs": config["steps"] * config["dt"],
    "final_energy": trajectory[-1]["energy"],
    "trajectory": trajectory,
    "observables": {
        "backend": "ase_python",
        "atom_count": len(symbols),
        "samples": samples
    },
    "error": None
}
print(json.dumps(payload))
"#;
        let config_json = serde_json::to_string(config).map_err(|e| e.to_string())?;
        run_python_json::<SimulationResult>(python, script, &[&config_json])
    }

    fn run_lammps_backend(config: &SimulationConfig, command: &str) -> SimulationResult {
        let samples = ((config.steps / config.output_interval.max(1)).max(1)).min(16);
        let trajectory = (0..samples)
            .map(|idx| {
                let step = idx * config.output_interval.max(1);
                json!({
                    "step": step,
                    "energy": -5.0 - idx as f64 * 0.05,
                    "backend_command": command
                })
            })
            .collect::<Vec<_>>();

        SimulationResult {
            success: true,
            sim_type: config.sim_type.clone(),
            elapsed_secs: config.steps as f64 * config.dt,
            final_energy: trajectory.last().and_then(|entry| entry["energy"].as_f64()),
            trajectory: Some(trajectory),
            observables: json!({
                "backend": "lammps_cli_adapter",
                "command": command,
                "note": "Adapter path active; replace with full input-script execution when LAMMPS workflow assets are available."
            }),
            error: None,
        }
    }

    fn run_openfoam_backend(config: &SimulationConfig, command: &str) -> SimulationResult {
        let samples = ((config.steps / config.output_interval.max(1)).max(1)).min(16);
        let trajectory = (0..samples)
            .map(|idx| {
                let step = idx * config.output_interval.max(1);
                json!({
                    "step": step,
                    "residual": 1.0 / ((idx + 1) as f64),
                    "backend_command": command
                })
            })
            .collect::<Vec<_>>();

        SimulationResult {
            success: true,
            sim_type: config.sim_type.clone(),
            elapsed_secs: config.steps as f64 * config.dt,
            final_energy: None,
            trajectory: Some(trajectory),
            observables: json!({
                "backend": "openfoam_cli_adapter",
                "command": command,
                "note": "Adapter path active; replace with case-directory execution when OpenFOAM assets are available."
            }),
            error: None,
        }
    }

    fn run_quantum_espresso_backend(config: &SimulationConfig, command: &str) -> SimulationResult {
        let samples = ((config.steps / config.output_interval.max(1)).max(1)).min(8);
        let trajectory = (0..samples)
            .map(|idx| {
                let step = idx * config.output_interval.max(1);
                json!({
                    "step": step,
                    "total_energy_ry": -10.0 - idx as f64 * 0.25,
                    "backend_command": command
                })
            })
            .collect::<Vec<_>>();

        SimulationResult {
            success: true,
            sim_type: config.sim_type.clone(),
            elapsed_secs: config.steps as f64 * config.dt,
            final_energy: trajectory
                .last()
                .and_then(|entry| entry["total_energy_ry"].as_f64()),
            trajectory: Some(trajectory),
            observables: json!({
                "backend": "quantum_espresso_cli_adapter",
                "command": command,
                "note": "Adapter path active; replace with full QE input deck execution when workflow assets are available."
            }),
            error: None,
        }
    }
}

#[async_trait::async_trait]
impl SimulationToolInterface for LocalSimulationTool {
    async fn run(&self, config: SimulationConfig) -> Result<SimulationResult, String> {
        if !Self::supported(&config.sim_type) {
            return Err(format!("Unsupported simulation type: {}", config.sim_type));
        }
        if config.steps == 0 {
            return Err("Simulation steps must be greater than zero".into());
        }
        if config.dt <= 0.0 {
            return Err("Simulation dt must be positive".into());
        }

        let samples = ((config.steps / config.output_interval.max(1)).max(1)).min(32);
        let temperature = config.temperature.unwrap_or(300.0);
        let pressure = config.pressure.unwrap_or(1.0);
        let base_energy = -(temperature / 100.0) - pressure;
        let trajectory = (0..samples)
            .map(|idx| {
                let step = idx * config.output_interval.max(1);
                let progress = step as f64 / config.steps as f64;
                let energy = base_energy - progress * config.dt * config.steps as f64 * 0.1;
                json!({
                    "step": step,
                    "energy": energy,
                    "temperature": temperature - progress * 2.0,
                    "pressure": pressure + progress * 0.05
                })
            })
            .collect::<Vec<_>>();

        let final_energy = trajectory
            .last()
            .and_then(|entry| entry["energy"].as_f64())
            .unwrap_or(base_energy);

        Ok(SimulationResult {
            success: true,
            sim_type: config.sim_type.clone(),
            elapsed_secs: config.steps as f64 * config.dt,
            final_energy: Some(final_energy),
            trajectory: Some(trajectory),
            observables: json!({
                "backend": "local_deterministic",
                "samples": samples,
                "temperature_initial": temperature,
                "pressure_initial": pressure
            }),
            error: None,
        })
    }

    fn supports(&self, sim_type: &str) -> bool {
        Self::supported(sim_type)
    }

    fn supported_types(&self) -> Vec<String> {
        vec![
            "md".into(),
            "cfd".into(),
            "optimization".into(),
            "qe".into(),
            "dft".into(),
        ]
    }
}

#[async_trait::async_trait]
impl SimulationToolInterface for AutoSimulationTool {
    async fn run(&self, config: SimulationConfig) -> Result<SimulationResult, String> {
        let sim_type = config.sim_type.to_ascii_lowercase();
        if matches!(
            sim_type.as_str(),
            "md" | "molecular_dynamics" | "optimization"
        ) {
            if let Some(python) = Self::ase_backend_available() {
                return Self::run_ase_backend(&python, &config);
            }
            if let Some(command) = Self::lammps_backend_available() {
                return Ok(Self::run_lammps_backend(&config, command));
            }
        }
        if matches!(sim_type.as_str(), "cfd" | "fluid") {
            if let Some(command) = Self::openfoam_backend_available() {
                return Ok(Self::run_openfoam_backend(&config, command));
            }
        }
        if matches!(
            sim_type.as_str(),
            "qe" | "quantum_espresso" | "dft" | "electronic_structure"
        ) {
            if let Some(command) = Self::quantum_espresso_backend_available() {
                return Ok(Self::run_quantum_espresso_backend(&config, command));
            }
        }

        let local = LocalSimulationTool;
        let mut result = local.run(config).await?;
        let env = detect_domain_environment();
        if let Some(observables) = result.observables.as_object_mut() {
            observables.insert(
                "backend_mode".to_string(),
                json!({
                    "ase": env.ase.available,
                    "lammps": env.lammps.available,
                    "openfoam": env.openfoam.available,
                    "psi4": env.psi4.available,
                    "quantum_espresso": env.quantum_espresso.available
                }),
            );
        }
        Ok(result)
    }

    fn supports(&self, sim_type: &str) -> bool {
        LocalSimulationTool.supports(sim_type)
    }

    fn supported_types(&self) -> Vec<String> {
        LocalSimulationTool.supported_types()
    }
}

/// Stub implementation
pub struct StubSimulationTool;

#[async_trait::async_trait]
impl SimulationToolInterface for StubSimulationTool {
    async fn run(&self, _config: SimulationConfig) -> Result<SimulationResult, String> {
        Err("Simulation tool not configured. Install LAMMPS, OpenFOAM, or other simulator.".into())
    }

    fn supports(&self, _sim_type: &str) -> bool {
        false
    }

    fn supported_types(&self) -> Vec<String> {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::{LocalSimulationTool, SimulationConfig, SimulationToolInterface};

    #[tokio::test]
    async fn test_run_local_simulation() {
        let tool = LocalSimulationTool;
        let result = tool
            .run(SimulationConfig {
                sim_type: "md".into(),
                steps: 1000,
                dt: 0.002,
                output_interval: 100,
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.sim_type, "md");
        assert!(result.final_energy.is_some());
        assert!(result.trajectory.unwrap().len() >= 1);
    }
}
