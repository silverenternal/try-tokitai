//! Domain science tools for chemistry, biology, and simulation.
//!
//! These tools use local fallback implementations from `ai-scientist-science`
//! so the platform can execute real domain computations even without the
//! heavyweight external SDKs installed.

use ai_scientist_science::biology::{AutoBiologyTool, BiologyToolInterface};
use ai_scientist_science::chemistry::{AutoChemistryTool, ChemistryToolInterface};
use ai_scientist_science::detect_domain_environment;
use ai_scientist_science::simulation::{AutoSimulationTool, SimulationConfig, SimulationToolInterface};
use serde_json::{json, Value};
use tokio::runtime::Builder;
use tokitai::tool;

pub struct DomainScienceTools;

fn block_on_domain<F, T>(future: F) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, String>>,
{
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.block_on(future)
    } else {
        Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Failed to build Tokio runtime: {}", e))?
            .block_on(future)
    }
}

fn build_simulation_config(
    sim_type: String,
    steps: u64,
    dt: f64,
    temperature: Option<f64>,
    pressure: Option<f64>,
    system: Option<Value>,
    output_interval: Option<u64>,
    parameters: Option<Value>,
) -> SimulationConfig {
    SimulationConfig {
        sim_type,
        steps,
        dt,
        temperature,
        pressure,
        system: system.unwrap_or(Value::Null),
        output_interval: output_interval.unwrap_or(100),
        parameters: parameters.unwrap_or(Value::Null),
    }
}

#[tool]
impl DomainScienceTools {
    /// Calculate molecular weight from a SMILES string.
    pub fn chemistry_mol_weight(&self, smiles: String) -> Result<Value, String> {
        let tool = AutoChemistryTool;
        let weight = block_on_domain(tool.mol_weight(&smiles))?;
        Ok(json!({
            "status": "success",
            "backend": "local_heuristic",
            "operation": "chemistry_mol_weight",
            "smiles": smiles,
            "molecular_weight": weight
        }))
    }

    /// Calculate local heuristic descriptors from a SMILES string.
    pub fn chemistry_descriptors(&self, smiles: String) -> Result<Value, String> {
        let tool = AutoChemistryTool;
        let descriptors = block_on_domain(tool.descriptors(&smiles))?;
        Ok(json!({
            "status": "success",
            "operation": "chemistry_descriptors",
            "smiles": smiles,
            "descriptors": descriptors
        }))
    }

    /// Generate molecular conformers, preferring RDKit when available.
    pub fn chemistry_conformers(&self, smiles: String, num: Option<usize>) -> Result<Value, String> {
        let tool = AutoChemistryTool;
        let conformers = block_on_domain(tool.generate_conformers(&smiles, num.unwrap_or(1)))?;
        Ok(json!({
            "status": "success",
            "operation": "chemistry_conformers",
            "smiles": smiles,
            "conformers": conformers
        }))
    }

    /// Calculate local fingerprint similarity between two SMILES strings.
    pub fn chemistry_similarity(&self, smiles_a: String, smiles_b: String) -> Result<Value, String> {
        let tool = AutoChemistryTool;
        let similarity = block_on_domain(tool.similarity(&smiles_a, &smiles_b))?;
        Ok(json!({
            "status": "success",
            "operation": "chemistry_similarity",
            "smiles_a": smiles_a,
            "smiles_b": smiles_b,
            "similarity": similarity
        }))
    }

    /// Evaluate a lightweight quantum chemistry energy, preferring Psi4 when available.
    pub fn chemistry_quantum_energy(
        &self,
        structure: Value,
        method: Option<String>,
    ) -> Result<Value, String> {
        let tool = AutoChemistryTool;
        let result = block_on_domain(tool.quantum_energy(&structure, method.as_deref()))?;
        Ok(json!({
            "status": "success",
            "operation": "chemistry_quantum_energy",
            "result": result
        }))
    }

    /// Translate a DNA sequence into amino acids.
    pub fn biology_translate(&self, sequence: String) -> Result<Value, String> {
        let tool = AutoBiologyTool;
        let protein = block_on_domain(tool.translate(&sequence))?;
        Ok(json!({
            "status": "success",
            "operation": "biology_translate",
            "sequence": sequence,
            "protein": protein
        }))
    }

    /// Compute reverse complement and GC content for a DNA sequence.
    pub fn biology_sequence_analysis(&self, sequence: String) -> Result<Value, String> {
        let tool = AutoBiologyTool;
        let reverse_complement = block_on_domain(tool.reverse_complement(&sequence))?;
        let gc_content = block_on_domain(tool.gc_content(&sequence))?;
        Ok(json!({
            "status": "success",
            "operation": "biology_sequence_analysis",
            "sequence": sequence,
            "reverse_complement": reverse_complement,
            "gc_content": gc_content
        }))
    }

    /// Align two sequences with a local Needleman-Wunsch implementation.
    pub fn biology_align(&self, seq_a: String, seq_b: String) -> Result<Value, String> {
        let tool = AutoBiologyTool;
        let alignment = block_on_domain(tool.align(&seq_a, &seq_b))?;
        Ok(json!({
            "status": "success",
            "operation": "biology_align",
            "alignment": alignment
        }))
    }

    /// Run BLAST-style validation path; prefers Biopython if available and otherwise reports local fallback status.
    pub fn biology_blast(&self, sequence: String, database: String) -> Result<Value, String> {
        let tool = AutoBiologyTool;
        let result = block_on_domain(tool.blast(&sequence, &database))?;
        Ok(json!({
            "status": "success",
            "operation": "biology_blast",
            "result": result
        }))
    }

    /// Run a deterministic local simulation useful for workflow validation.
    pub fn simulation_run(
        &self,
        sim_type: String,
        steps: u64,
        dt: f64,
        temperature: Option<f64>,
        pressure: Option<f64>,
        system: Option<Value>,
        output_interval: Option<u64>,
        parameters: Option<Value>,
    ) -> Result<Value, String> {
        let tool = AutoSimulationTool;
        let result = block_on_domain(tool.run(build_simulation_config(
            sim_type,
            steps,
            dt,
            temperature,
            pressure,
            system,
            output_interval,
            parameters,
        )))?;

        Ok(json!({
            "status": "success",
            "operation": "simulation_run",
            "result": result
        }))
    }

    /// List supported local simulation types.
    pub fn simulation_supported_types(&self) -> Result<Value, String> {
        let tool = AutoSimulationTool;
        Ok(json!({
            "status": "success",
            "operation": "simulation_supported_types",
            "types": tool.supported_types()
        }))
    }

    /// Run a simulation with a backend-oriented preset for materials or CFD workflows.
    pub fn simulation_run_preset(
        &self,
        preset: String,
        steps: Option<u64>,
        dt: Option<f64>,
        system: Option<Value>,
        parameters: Option<Value>,
    ) -> Result<Value, String> {
        let preset_lower = preset.to_ascii_lowercase();
        let sim_type = match preset_lower.as_str() {
            "ase" | "lammps" | "md" | "materials" => "md",
            "openfoam" | "cfd" | "fluid" => "cfd",
            "qe" | "quantum_espresso" | "dft" => "qe",
            other => other,
        }
        .to_string();

        self.simulation_run(
            sim_type,
            steps.unwrap_or(1000),
            dt.unwrap_or(0.001),
            Some(300.0),
            Some(1.0),
            system,
            Some(100),
            parameters,
        )
    }

    /// Report environment and backend availability for scientific SDKs/CLIs.
    pub fn scientific_backend_status(&self) -> Result<Value, String> {
        Ok(json!({
            "status": "success",
            "operation": "scientific_backend_status",
            "report": detect_domain_environment()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::DomainScienceTools;
    use serde_json::json;
    use tokitai::ToolProvider;

    #[test]
    fn test_tool_registration() {
        let defs = DomainScienceTools::tool_definitions();
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"chemistry_mol_weight"));
        assert!(names.contains(&"chemistry_conformers"));
        assert!(names.contains(&"chemistry_quantum_energy"));
        assert!(names.contains(&"biology_translate"));
        assert!(names.contains(&"biology_blast"));
        assert!(names.contains(&"simulation_run"));
        assert!(names.contains(&"simulation_run_preset"));
        assert!(names.contains(&"scientific_backend_status"));
    }

    #[test]
    fn test_chemistry_biology_and_simulation_calls() {
        let tool = DomainScienceTools;

        let chemistry = tool
            .call_tool("chemistry_mol_weight", &json!({ "smiles": "CCO" }))
            .unwrap();
        assert_eq!(chemistry["status"], "success");
        assert!(chemistry["molecular_weight"].as_f64().unwrap() > 40.0);

        let conformers = tool
            .call_tool(
                "chemistry_conformers",
                &json!({ "smiles": "CCO", "num": 2 }),
            )
            .unwrap();
        assert_eq!(conformers["status"], "success");
        assert_eq!(conformers["conformers"].as_array().unwrap().len(), 2);

        let quantum = tool
            .call_tool(
                "chemistry_quantum_energy",
                &json!({
                    "structure": {
                        "symbols": ["H", "H"],
                        "positions": [[0.0, 0.0, 0.0], [0.0, 0.0, 0.74]]
                    },
                    "method": "scf/sto-3g"
                }),
            )
            .unwrap();
        assert_eq!(quantum["status"], "success");

        let biology = tool
            .call_tool("biology_translate", &json!({ "sequence": "ATGGCC" }))
            .unwrap();
        assert_eq!(biology["protein"], "MA");

        let blast = tool
            .call_tool(
                "biology_blast",
                &json!({ "sequence": "ATGGCC", "database": "local-demo" }),
            )
            .unwrap();
        assert_eq!(blast["status"], "success");
        assert!(blast["result"]["backend"].is_string() || blast["result"]["backend_mode"].is_string());

        let simulation = tool
            .call_tool(
                "simulation_run",
                &json!({
                    "sim_type": "md",
                    "steps": 1000,
                    "dt": 0.001
                }),
            )
            .unwrap();
        assert_eq!(simulation["status"], "success");
        assert_eq!(simulation["result"]["success"], true);

        let preset = tool
            .call_tool(
                "simulation_run_preset",
                &json!({
                    "preset": "qe",
                    "steps": 200
                }),
            )
            .unwrap();
        assert_eq!(preset["status"], "success");

        let status = tool.call_tool("scientific_backend_status", &json!({})).unwrap();
        assert_eq!(status["status"], "success");
        assert!(status["report"]["rdkit"]["name"].is_string());
    }
}
