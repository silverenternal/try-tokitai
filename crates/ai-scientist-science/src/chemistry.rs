//! Chemistry Tool Interface
//!
//! Trait-based abstraction for chemistry computations.
//! Includes a local heuristic backend so the platform can run common
//! cheminformatics tasks without waiting for RDKit/Psi4 installation.

use std::collections::{BTreeMap, HashMap, HashSet};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::python_bridge::{find_python_with_module, run_python_json};

/// Chemistry computation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChemistryResult {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

/// Chemistry Tool trait - implement for specific backends
#[async_trait::async_trait]
pub trait ChemistryToolInterface: Send + Sync {
    /// Calculate molecular weight from SMILES
    async fn mol_weight(&self, smiles: &str) -> Result<f64, String>;

    /// Calculate molecular similarity (Tanimoto)
    async fn similarity(&self, smiles_a: &str, smiles_b: &str) -> Result<f64, String>;

    /// Generate 3D conformers
    async fn generate_conformers(&self, smiles: &str, num: usize) -> Result<Vec<String>, String>;

    /// Calculate molecular descriptors
    async fn descriptors(&self, smiles: &str) -> Result<serde_json::Value, String>;

    /// Run a SMILES-based reaction
    async fn reaction(&self, reactants: &[String], reaction_smarts: &str) -> Result<Vec<String>, String>;

    /// Run a lightweight quantum chemistry energy evaluation.
    async fn quantum_energy(&self, structure: &serde_json::Value, method: Option<&str>) -> Result<serde_json::Value, String>;
}

/// Local chemistry implementation for fast, dependency-light fallback.
pub struct LocalChemistryTool;

impl LocalChemistryTool {
    fn atomic_weights() -> HashMap<&'static str, f64> {
        HashMap::from([
            ("H", 1.008),
            ("B", 10.81),
            ("C", 12.011),
            ("N", 14.007),
            ("O", 15.999),
            ("F", 18.998),
            ("P", 30.974),
            ("S", 32.06),
            ("Cl", 35.45),
            ("Br", 79.904),
            ("I", 126.904),
        ])
    }

    fn tokenize_smiles(smiles: &str) -> Result<Vec<String>, String> {
        if smiles.trim().is_empty() {
            return Err("SMILES cannot be empty".into());
        }

        let chars: Vec<char> = smiles.trim().chars().collect();
        let mut i = 0usize;
        let mut atoms = Vec::new();

        while i < chars.len() {
            let c = chars[i];
            if c == '[' {
                let mut j = i + 1;
                while j < chars.len() && chars[j] != ']' {
                    j += 1;
                }
                if j >= chars.len() {
                    return Err("Unclosed bracket in SMILES".into());
                }
                let token = chars[i + 1..j].iter().collect::<String>();
                let element: String = token
                    .chars()
                    .take_while(|ch| ch.is_ascii_alphabetic())
                    .collect();
                if !element.is_empty() {
                    atoms.push(element);
                }
                i = j + 1;
                continue;
            }

            if c.is_ascii_uppercase() {
                if i + 1 < chars.len() && chars[i + 1].is_ascii_lowercase() {
                    atoms.push(format!("{}{}", c, chars[i + 1]));
                    i += 2;
                } else {
                    atoms.push(c.to_string());
                    i += 1;
                }
                continue;
            }

            if c.is_ascii_lowercase() {
                atoms.push(c.to_ascii_uppercase().to_string());
            }

            i += 1;
        }

        if atoms.is_empty() {
            return Err("No recognizable atoms found in SMILES".into());
        }

        Ok(atoms)
    }

    fn atom_counts(smiles: &str) -> Result<BTreeMap<String, usize>, String> {
        let mut counts = BTreeMap::new();
        for atom in Self::tokenize_smiles(smiles)? {
            *counts.entry(atom).or_insert(0) += 1;
        }
        Ok(counts)
    }

    fn fingerprint(smiles: &str) -> Result<HashSet<String>, String> {
        let chars: Vec<char> = smiles.trim().chars().collect();
        if chars.is_empty() {
            return Err("SMILES cannot be empty".into());
        }

        let mut fp = HashSet::new();
        for atom in Self::tokenize_smiles(smiles)? {
            fp.insert(format!("atom:{atom}"));
        }

        for window in chars.windows(2) {
            let pattern = window.iter().collect::<String>();
            fp.insert(format!("pair:{pattern}"));
        }

        Ok(fp)
    }
}

pub struct AutoChemistryTool;

impl AutoChemistryTool {
    fn python_backend_available() -> Option<String> {
        find_python_with_module("rdkit")
    }

    fn rdkit_mol_weight(python: &str, smiles: &str) -> Result<f64, String> {
        let script = r#"
import json
import sys
from rdkit import Chem
from rdkit.Chem import Descriptors

smiles = sys.argv[1]
mol = Chem.MolFromSmiles(smiles)
if mol is None:
    raise SystemExit("Invalid SMILES")
print(json.dumps(Descriptors.MolWt(mol)))
"#;
        run_python_json::<f64>(python, script, &[smiles])
    }

    fn rdkit_similarity(python: &str, smiles_a: &str, smiles_b: &str) -> Result<f64, String> {
        let script = r#"
import json
import sys
from rdkit import Chem, DataStructs
from rdkit.Chem import AllChem

smiles_a = sys.argv[1]
smiles_b = sys.argv[2]
mol_a = Chem.MolFromSmiles(smiles_a)
mol_b = Chem.MolFromSmiles(smiles_b)
if mol_a is None or mol_b is None:
    raise SystemExit("Invalid SMILES")
fp_a = AllChem.GetMorganFingerprintAsBitVect(mol_a, 2, nBits=2048)
fp_b = AllChem.GetMorganFingerprintAsBitVect(mol_b, 2, nBits=2048)
print(json.dumps(DataStructs.TanimotoSimilarity(fp_a, fp_b)))
"#;
        run_python_json::<f64>(python, script, &[smiles_a, smiles_b])
    }

    fn rdkit_descriptors(python: &str, smiles: &str) -> Result<serde_json::Value, String> {
        let script = r#"
import json
import sys
from rdkit import Chem
from rdkit.Chem import Descriptors, Lipinski

smiles = sys.argv[1]
mol = Chem.MolFromSmiles(smiles)
if mol is None:
    raise SystemExit("Invalid SMILES")

payload = {
    "backend": "rdkit_python",
    "molecular_weight": Descriptors.MolWt(mol),
    "logp": Descriptors.MolLogP(mol),
    "tpsa": Descriptors.TPSA(mol),
    "h_donors": Lipinski.NumHDonors(mol),
    "h_acceptors": Lipinski.NumHAcceptors(mol),
    "rotatable_bonds": Lipinski.NumRotatableBonds(mol),
    "heavy_atom_count": mol.GetNumHeavyAtoms(),
}
print(json.dumps(payload))
"#;
        run_python_json::<serde_json::Value>(python, script, &[smiles])
    }

    fn rdkit_conformers(python: &str, smiles: &str, num: usize) -> Result<Vec<String>, String> {
        let script = r#"
import json
import sys
from rdkit import Chem
from rdkit.Chem import AllChem

smiles = sys.argv[1]
num = int(sys.argv[2])
mol = Chem.MolFromSmiles(smiles)
if mol is None:
    raise SystemExit("Invalid SMILES")
mol = Chem.AddHs(mol)
ids = AllChem.EmbedMultipleConfs(mol, numConfs=num, randomSeed=42)
payload = []
for conf_id in ids:
    conf = mol.GetConformer(conf_id)
    atoms = []
    for atom_idx, atom in enumerate(mol.GetAtoms()):
        pos = conf.GetAtomPosition(atom_idx)
        atoms.append({
            "element": atom.GetSymbol(),
            "x": pos.x,
            "y": pos.y,
            "z": pos.z
        })
    payload.append(json.dumps({
        "conformer_id": int(conf_id),
        "backend": "rdkit_python",
        "atoms": atoms
    }))
print(json.dumps(payload))
"#;
        run_python_json::<Vec<String>>(python, script, &[smiles, &num.to_string()])
    }

    fn psi4_backend_available() -> Option<String> {
        find_python_with_module("psi4")
    }

    fn psi4_quantum_energy(
        python: &str,
        structure: &serde_json::Value,
        method: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let script = r#"
import json
import sys
import psi4

structure = json.loads(sys.argv[1])
method = sys.argv[2]
symbols = structure.get("symbols") or ["H", "H"]
positions = structure.get("positions") or [[0.0, 0.0, 0.0], [0.0, 0.0, 0.74]]
charge = structure.get("charge", 0)
multiplicity = structure.get("multiplicity", 1)

geom_lines = [f"{charge} {multiplicity}"]
for symbol, pos in zip(symbols, positions):
    geom_lines.append(f"{symbol} {pos[0]} {pos[1]} {pos[2]}")

molecule = psi4.geometry("\n".join(geom_lines))
energy = psi4.energy(method, molecule=molecule)

print(json.dumps({
    "backend": "psi4_python",
    "method": method,
    "energy_hartree": energy,
    "atom_count": len(symbols)
}))
"#;
        let structure_json = serde_json::to_string(structure).map_err(|e| e.to_string())?;
        run_python_json::<serde_json::Value>(python, script, &[&structure_json, method.unwrap_or("scf/sto-3g")])
    }
}

#[async_trait::async_trait]
impl ChemistryToolInterface for LocalChemistryTool {
    async fn mol_weight(&self, smiles: &str) -> Result<f64, String> {
        let weights = Self::atomic_weights();
        let counts = Self::atom_counts(smiles)?;
        let mut total = 0.0;

        for (atom, count) in counts {
            let weight = weights
                .get(atom.as_str())
                .ok_or_else(|| format!("Unsupported atom in local chemistry backend: {}", atom))?;
            total += weight * count as f64;
        }

        Ok(total)
    }

    async fn similarity(&self, smiles_a: &str, smiles_b: &str) -> Result<f64, String> {
        let fp_a = Self::fingerprint(smiles_a)?;
        let fp_b = Self::fingerprint(smiles_b)?;

        let intersection = fp_a.intersection(&fp_b).count() as f64;
        let union = fp_a.union(&fp_b).count() as f64;
        if union == 0.0 {
            return Ok(0.0);
        }
        Ok(intersection / union)
    }

    async fn generate_conformers(&self, smiles: &str, num: usize) -> Result<Vec<String>, String> {
        let atoms = Self::tokenize_smiles(smiles)?;
        let count = num.max(1).min(16);
        let conformers = (0..count)
            .map(|idx| {
                let coords = atoms
                    .iter()
                    .enumerate()
                    .map(|(atom_idx, atom)| {
                        json!({
                            "element": atom,
                            "x": atom_idx as f64 * 1.25,
                            "y": idx as f64 * 0.35,
                            "z": ((atom_idx + idx) % 3) as f64 * 0.2
                        })
                    })
                    .collect::<Vec<_>>();
                json!({
                    "conformer_id": idx,
                    "backend": "local_heuristic",
                    "atoms": coords
                })
                .to_string()
            })
            .collect();

        Ok(conformers)
    }

    async fn descriptors(&self, smiles: &str) -> Result<serde_json::Value, String> {
        let counts = Self::atom_counts(smiles)?;
        let atom_count: usize = counts.values().sum();
        let unique_atoms = counts.len();
        let molecular_weight = self.mol_weight(smiles).await?;
        let heavy_atoms: usize = counts
            .iter()
            .filter(|(atom, _)| atom.as_str() != "H")
            .map(|(_, count)| *count)
            .sum();
        let hetero_atoms: usize = counts
            .iter()
            .filter(|(atom, _)| !matches!(atom.as_str(), "C" | "H"))
            .map(|(_, count)| *count)
            .sum();

        Ok(json!({
            "backend": "local_heuristic",
            "atom_count": atom_count,
            "unique_atoms": unique_atoms,
            "heavy_atom_count": heavy_atoms,
            "hetero_atom_count": hetero_atoms,
            "molecular_weight": molecular_weight,
            "atom_counts": counts
        }))
    }

    async fn reaction(&self, reactants: &[String], reaction_smarts: &str) -> Result<Vec<String>, String> {
        if reactants.is_empty() {
            return Err("At least one reactant is required".into());
        }

        for reactant in reactants {
            let _ = Self::tokenize_smiles(reactant)?;
        }

        let joined = reactants.join(".");
        Ok(vec![format!("{joined}>>{reaction_smarts}")])
    }

    async fn quantum_energy(&self, structure: &serde_json::Value, method: Option<&str>) -> Result<serde_json::Value, String> {
        let atom_count = structure
            .get("symbols")
            .and_then(|v| v.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0);
        let estimated = -(atom_count.max(1) as f64) * 0.5;
        Ok(json!({
            "backend": "local_quantum_fallback",
            "method": method.unwrap_or("heuristic"),
            "energy_hartree": estimated,
            "atom_count": atom_count,
            "note": "Heuristic fallback; install Psi4 for ab initio energy evaluation."
        }))
    }
}

#[async_trait::async_trait]
impl ChemistryToolInterface for AutoChemistryTool {
    async fn mol_weight(&self, smiles: &str) -> Result<f64, String> {
        if let Some(python) = Self::python_backend_available() {
            return Self::rdkit_mol_weight(&python, smiles);
        }
        let local = LocalChemistryTool;
        local.mol_weight(smiles).await
    }

    async fn similarity(&self, smiles_a: &str, smiles_b: &str) -> Result<f64, String> {
        if let Some(python) = Self::python_backend_available() {
            return Self::rdkit_similarity(&python, smiles_a, smiles_b);
        }
        let local = LocalChemistryTool;
        local.similarity(smiles_a, smiles_b).await
    }

    async fn generate_conformers(&self, smiles: &str, num: usize) -> Result<Vec<String>, String> {
        if let Some(python) = Self::python_backend_available() {
            return Self::rdkit_conformers(&python, smiles, num);
        }
        let local = LocalChemistryTool;
        local.generate_conformers(smiles, num).await
    }

    async fn descriptors(&self, smiles: &str) -> Result<serde_json::Value, String> {
        if let Some(python) = Self::python_backend_available() {
            return Self::rdkit_descriptors(&python, smiles);
        }
        let local = LocalChemistryTool;
        local.descriptors(smiles).await
    }

    async fn reaction(&self, reactants: &[String], reaction_smarts: &str) -> Result<Vec<String>, String> {
        let local = LocalChemistryTool;
        local.reaction(reactants, reaction_smarts).await
    }

    async fn quantum_energy(&self, structure: &serde_json::Value, method: Option<&str>) -> Result<serde_json::Value, String> {
        if let Some(python) = Self::psi4_backend_available() {
            return Self::psi4_quantum_energy(&python, structure, method);
        }
        let local = LocalChemistryTool;
        local.quantum_energy(structure, method).await
    }
}

/// Stub implementation for when chemistry tools aren't available
pub struct StubChemistryTool;

#[async_trait::async_trait]
impl ChemistryToolInterface for StubChemistryTool {
    async fn mol_weight(&self, _smiles: &str) -> Result<f64, String> {
        Err("Chemistry tool not configured. Install RDKit or OpenBabel.".into())
    }

    async fn similarity(&self, _smiles_a: &str, _smiles_b: &str) -> Result<f64, String> {
        Err("Chemistry tool not configured.".into())
    }

    async fn generate_conformers(&self, _smiles: &str, _num: usize) -> Result<Vec<String>, String> {
        Err("Chemistry tool not configured.".into())
    }

    async fn descriptors(&self, _smiles: &str) -> Result<serde_json::Value, String> {
        Err("Chemistry tool not configured.".into())
    }

    async fn reaction(&self, _reactants: &[String], _reaction_smarts: &str) -> Result<Vec<String>, String> {
        Err("Chemistry tool not configured.".into())
    }

    async fn quantum_energy(&self, _structure: &serde_json::Value, _method: Option<&str>) -> Result<serde_json::Value, String> {
        Err("Quantum chemistry backend not configured.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::{ChemistryToolInterface, LocalChemistryTool};

    #[tokio::test]
    async fn test_mol_weight_and_descriptors() {
        let tool = LocalChemistryTool;
        let weight = tool.mol_weight("CCO").await.unwrap();
        assert!((weight - 40.021).abs() < 0.05);

        let descriptors = tool.descriptors("CCO").await.unwrap();
        assert_eq!(descriptors["atom_count"], 3);
        assert_eq!(descriptors["unique_atoms"], 2);
    }

    #[tokio::test]
    async fn test_similarity_and_conformers() {
        let tool = LocalChemistryTool;
        let sim = tool.similarity("CCO", "CCN").await.unwrap();
        assert!(sim > 0.2);
        assert!(sim < 1.0);

        let conformers = tool.generate_conformers("CCO", 2).await.unwrap();
        assert_eq!(conformers.len(), 2);
        assert!(conformers[0].contains("local_heuristic"));
    }
}
